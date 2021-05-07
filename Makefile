.PHONY: help
help:  ## Displays this message
	@fgrep -h "##" $(MAKEFILE_LIST) | fgrep -v fgrep | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[33m%-30s\033[0m %s\n", $$1, $$2}'

clear:  ## Removes all docker volumes
	-docker volume rm redis_data

.PHONY: builder
builder:  ## Create docker image to build linux binaries
	docker build -t cargobuilder .

CARGO_CMD=docker run -it --rm \
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

.PHONY: redis-darwin
redis-darwin: ## Runs real redis locally on MacOSX
	cargo build --release
	redis-server --port 6666 --loadmodule ./target/release/libredelay.dylib

.PHONY: redis-linux
redis-linux: build ## Runs real redis inside a docker container
	docker run -it --rm \
		-p 6666:6379 \
		-v cargobuilder_target:/target  \
		-v redis_data:/data \
		redis redis-server --appendonly yes --loadmodule /target/release/libredelay.so

.PHONY: test
test:
	cargo t --features test
