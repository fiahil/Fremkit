SHELL := /bin/bash
.PHONY: help lint loom test bench

help:			## Show this help
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z0-9_-]+:.*?## / {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

lint:			## Run clippy
	cargo clippy

loom:			## Run tests with loom
	RUSTFLAGS="--cfg loom" \
	LOOM_MAX_PREEMPTIONS=3 \
	cargo test --release

test:			## Run tests
	cargo test

bench:			## Run benchmarks
	cargo bench

all:			## Run all checks
all: lint loom test bench
