SHELL := /bin/bash
.PHONY: help lint loom test bench

help:			## Show this help
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z0-9_-]+:.*?## / {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

lint:			## Run clippy
	cargo clippy

loom:			## Run tests with loom
	RUSTFLAGS="--cfg loom" \
	LOOM_MAX_PREEMPTIONS=2 \
	cargo test log::bounded::test::test_loom

test:			## Run tests
	cargo test

bench:			## Run benchmarks
	@mv dist/benchmark target/criterion 2> /dev/null || true
	cargo bench -- --sample-size 500 --noise-threshold 0.05
	mv target/criterion dist/benchmark

all:			## Run all tests and checks
all: lint loom test bench
