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

CARGO_CMD_EXTRA=
CARGO_CMD=${CRUN} run ${IT} --rm \
	-v cargobuilder_home_cargo:/opt/cargo \
	--user ${CUSER} \
	-e CARGO_TARGET_DIR=.container_target \
	-e INTEGRATION_TEST_REDIS_HOST=${INTEGRATION_TEST_REDIS_HOST} \
	-e INTEGRATION_TEST_REDIS_PORT=${INTEGRATION_TEST_REDIS_PORT} \
	-v ${PWD}:/usr/src/myapp:Z \
	${CARGO_CMD_EXTRA} \
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
	-docker-compose -f docker-compose.cluster.yaml -p redelay_cluster down

#
# Cargo
#
.PHONY: builder
builder:  ## Create docker image to build linux binaries
	${CRUN} build -t cargobuilder .

.PHONY: build
build: ## Build linux binaries
	${CARGO_CMD} cargo build --release

.PHONY: builder-shell
builder-shell: ## Drop in builder container shell
	${CARGO_CMD} sh

.PHONY: test
test: ## Run unit tests
	cargo t --features test

#
# Services for integration
#
.PHONY: redis-server
redis-server: build ## Run redis with redelay
	${REDIS_SERVER} --port ${INTEGRATION_TEST_REDIS_PORT} --appendonly yes --loadmodule /target/release/libredelay.so

.PHONY: redis-server
redis-server-d: build ## Run redis with redelay
	${REDIS_SERVER_D} --port ${INTEGRATION_TEST_REDIS_PORT} --appendonly yes --loadmodule /target/release/libredelay.so

.PHONY: stop-redis-server-d
stop-redis-server:  ##  Stop redis started by redis-server or redis-server-d
	${CRUN} stop redis-server-redelay

CLUSTER_COMPOSE_CMD=docker-compose -f docker-compose.cluster.yaml -p redelay_cluster

.PHONY: redis-cluster
redis-cluster: build  ## Start redis in cluster mode
	$(CLUSTER_COMPOSE_CMD) up

.PHONY: redis-cluster-d
redis-cluster-d: build  ## Same as redis-cluster but in background
	$(CLUSTER_COMPOSE_CMD) up -d
	@docker run --rm --network=redelay_cluster_default -e TARGETS=redis-node-0:6379,redis-node-1:6379,redis-node-2:6379,redis-node-3:6379,redis-node-4:6379,redis-node-5:6379, waisbrot/wait
	sleep 5 # =/

.PHONY: redis-cluster-logs
redis-cluster-logs:  ## Print and follow logs for all the redis cluster nodes
	$(CLUSTER_COMPOSE_CMD) logs --follow

.PHONY: stop-redis-cluster
stop-redis-cluster:  ## Stop servers started by redis-cluster or redis-cluster-d
	$(CLUSTER_COMPOSE_CMD) stop

#
# Integration test runners
#
.PHONY: test-integration
test-integration: ## Run integration tests (requires Redis running with the module)
	cargo t --features integration_test --test '*'

.ONESHELL: start-integration
.PHONY: start-integration
start-integration: ## Setup a redis instance and run integration tests against it
	@$(MAKE) redis-server-d
	function tearDown {
		@$(MAKE) stop-redis-server-d
	}
	@trap tearDown EXIT
	@$(MAKE) test-integration

#
# Integration test runners using Redis Cluster
#
.PHONY: test-cluster-integration
test-cluster-integration: INTEGRATION_TEST_REDIS_HOST=redis-node-5
test-cluster-integration: INTEGRATION_TEST_REDIS_PORT=6379
test-cluster-integration: CARGO_CMD_EXTRA += --network=redelay_cluster_default
test-cluster-integration: ## Setup a redis cluster and run cluster integration tests against it
	$(CARGO_CMD) cargo t --features integration_test,test_cluster --test '*' -- --nocapture

.ONESHELL: start-cluster-integration
.PHONY: start-cluster-integration
start-cluster-integration: ## Setup a redis cluster and run cluster integration tests against it
	@$(MAKE) redis-cluster-d
	function tearDown {
		@$(MAKE) stop-redis-cluster
	}
	@trap tearDown EXIT
	@$(MAKE) test-cluster-integration


#
# Integrity test runners using standalone Redis
#
.PHONY: test-integrity-setup
test-integrity-setup: ## Load data into redis for the integrity test (requires Redis running with the module)
	cargo t --features integrity_test_setup --test '*'

.PHONY: test-integrity
test-integrity: ## Run AOF/RDB integrity tests (requires Redis running with the module and data fed into it)
	cargo t --features integrity_test --test '*'

.ONESHELL: start-integrity
.PHONY: start-integrity
start-integrity: ## Setup a redis instance run integrity setup, restart the instance and run the integrity tests
	function tearDown {
		-@$(MAKE) stop-redis-server
	}
	@trap tearDown EXIT

	-${CRUN} volume rm redis_data
	@$(MAKE) redis-server-d
	@$(MAKE) test-integrity-setup
	@$(MAKE) stop-redis-server
	@$(MAKE) redis-server-d
	@$(MAKE) test-integrity


#
# Integrity test runners using Redis Cluster
#
.PHONY: test-cluster-integrity-setup
test-cluster-integrity-setup: INTEGRATION_TEST_REDIS_HOST=redis-node-5
test-cluster-integrity-setup: INTEGRATION_TEST_REDIS_PORT=6379
test-cluster-integrity-setup: CARGO_CMD_EXTRA += --network=redelay_cluster_default
test-cluster-integrity-setup: ## Run integrity checks in redis cluster
	$(CARGO_CMD) cargo t --features integrity_test_setup,test_cluster --test '*' -- --nocapture


.PHONY: test-cluster-integrity
test-cluster-integrity: INTEGRATION_TEST_REDIS_HOST=redis-node-5
test-cluster-integrity: INTEGRATION_TEST_REDIS_PORT=6379
test-cluster-integrity: CARGO_CMD_EXTRA += --network=redelay_cluster_default
test-cluster-integrity: ## Run integrity checks in standalone redis
	$(CARGO_CMD) cargo t --features integrity_test,test_cluster --test '*'

.ONESHELL: start-cluster-integrity
.PHONY: start-cluster-integrity
start-cluster-integrity: ## Setup redis cluster and run integrity checks against it
	function tearDown {
		-$(MAKE) stop-redis-cluster
	}
	@trap tearDown EXIT

	${CLUSTER_COMPOSE_CMD} down
	@$(MAKE) redis-cluster-d
	@$(MAKE) test-cluster-integrity-setup
	sleep 5
	@$(MAKE) stop-redis-cluster
	@$(MAKE) redis-cluster-d
	@$(MAKE) test-cluster-integrity


.ONESHELL: start-all
.PHONY: start-all
start-all: builder clear test start-integration start-cluster-integration
	@echo All tests passed
