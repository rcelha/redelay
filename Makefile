UID:=$(shell id -u)
GID:=$(shell id -g)
IT:=$(shell [ -t 0 ] && echo -it)

CRUN=docker
ifeq ($(CRUN),podman)
CUSER=root
else
CUSER=${UID}:${GID}
endif
REDIS_IMAGE=docker.io/library/redis

INTEGRATION_TEST_REDIS_HOST=127.0.0.1
export INTEGRATION_TEST_REDIS_HOST
INTEGRATION_TEST_REDIS_PORT=60666
export INTEGRATION_TEST_REDIS_PORT

CARGO_CMD=${CRUN} run ${IT} --rm \
	-v cargobuilder_home_cargo:/opt/cargo \
	--user ${CUSER} \
	-e CARGO_TARGET_DIR=.container_target \
	-v ${PWD}:/usr/src/myapp:Z \
	cargobuilder
REDIS_SERVER_D=${CRUN} run -d --rm \
	-p ${INTEGRATION_TEST_REDIS_PORT}:${INTEGRATION_TEST_REDIS_PORT} \
	-v redis_data:/data \
	-v ${PWD}/.container_target:/target:Z  \
	--name redis-server-redelay \
	${REDIS_IMAGE} redis-server
REDIS_SERVER=${CRUN} run ${IT} --rm \
	-p ${INTEGRATION_TEST_REDIS_PORT}:${INTEGRATION_TEST_REDIS_PORT} \
	-v redis_data:/data \
	-v ${PWD}/.container_target:/target:Z  \
	--name redis-server-redelay \
	${REDIS_IMAGE} redis-server

.PHONY: help
help:  ## Displays this message
	@fgrep -h "##" $(MAKEFILE_LIST) | fgrep -v fgrep | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[33m%-30s\033[0m %s\n", $$1, $$2}'

clear:  ## Removes all docker volumes
	-${CRUN} volume rm redis_data

.PHONY: builder
builder:  ## Create docker image to build linux binaries
	${CRUN} build -t cargobuilder .

.PHONY: build
build: ## Build linux binaries
	${CARGO_CMD} cargo build --release

.PHONY: builder-shell
builder-shell: ## Drop in builder container shell
	${CARGO_CMD} sh

.PHONY: redis-server
redis-server: build ## Run redis with redelay
	${REDIS_SERVER} --port ${INTEGRATION_TEST_REDIS_PORT} --appendonly yes --loadmodule /target/release/libredelay.so

.PHONY: redis-server
redis-server-d: build ## Run redis with redelay
	${REDIS_SERVER_D} --port ${INTEGRATION_TEST_REDIS_PORT} --appendonly yes --loadmodule /target/release/libredelay.so

.PHONY: test
test: ## Run unit tests
	cargo t --features test

.PHONY: test-integration
test-integration: ## Run integration tests (requires Redis running with the module)
	cargo t --features integration_tests --test '*'

.PHONY: stop-redis-server-d
stop-redis-server-d:
	${CRUN} stop redis-server-redelay

.ONESHELL: start-integration
.PHONY: start-integration
start-integration: ## Setup a redis instance and run integration tests against it
	@$(MAKE) redis-server-d
	function tearDown {
		@$(MAKE) stop-redis-server-d
	}
	@trap tearDown EXIT
	@$(MAKE) test-integration
