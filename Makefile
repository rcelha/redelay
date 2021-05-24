IT:=$(shell [ -t 0 ] && echo -it)

.PHONY: help
help:  ## Displays this message
	@fgrep -h "##" $(MAKEFILE_LIST) | fgrep -v fgrep | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[33m%-30s\033[0m %s\n", $$1, $$2}'

clear:  ## Removes all docker volumes
	-docker volume rm redis_data

.PHONY: builder
builder:  ## Create docker image to build linux binaries
	docker build -t cargobuilder .

CARGO_CMD=docker run ${IT} --rm \
	-v cargobuilder_target:/usr/src/myapp/target \
	-v cargobuilder_home_cargo:/root/.cargo \
	-v cargobuilder_cache_git:/usr/local/cargo/git \
	-v cargobuilder_cache_res:/usr/local/cargo/registry \
	-v cargobuilder_cache_too:/usr/local/rustup/toolchains \
	-v ${PWD}:/usr/src/myapp \
	cargobuilder

.PHONY: build
build: ## Build linux binaries
	${CARGO_CMD} cargo build --release

.PHONY: builder-shell
builder-shell: ## Drop in builder container shell
	${CARGO_CMD} sh

.PHONY: redis-linux
redis-linux: build ## Runs real redis inside a docker container
	docker run ${IT} --rm \
		-p 6666:6379 \
		-v cargobuilder_target:/target  \
		-v redis_data:/data \
		--name redis-server-redelay \
		redis redis-server --appendonly yes --loadmodule /target/release/libredelay.so

.PHONY: test
test: ## Run unit tests
	cargo t --features test

.PHONY: test-integration
test-integration: ## Run integration tests (requires Redis running with the module)
	cargo t --features integration_tests --test '*'

.PHONY: setup-integration
setup-integration: build
	docker run -d --rm \
		-p 6666:6379 \
		-v cargobuilder_target:/target  \
		-v redis_data:/data \
		--name redis-server-redelay \
		redis redis-server --appendonly yes --loadmodule /target/release/libredelay.so


.PHONY: teardown-integration
teardown-integration:
	docker stop redis-server-redelay

.ONESHELL: start-integration
.PHONY: start-integration
start-integration: ## Setup a redis instance and run integration tests against it
	@$(MAKE) setup-integration
	function tearDown {
		@$(MAKE) teardown-integration
	}
	@trap tearDown EXIT
	@$(MAKE) test-integration
