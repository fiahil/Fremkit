SHELL := /bin/bash
.PHONY: help lint loom test sanitizer bench

CRATES = fremkit-channel

help:			## Show this help
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z0-9_-]+:.*?## / {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

lint:			## Run clippy
	cargo clippy

loom:			## Run tests with loom
	RUSTFLAGS="--cfg loom" \
	LOOM_MAX_PREEMPTIONS=2 \
	cargo test --release -p ${CRATES}

test:			## Run tests
	cargo test

sanitizer:		## Run tests with sanitizer
	RUSTFLAGS="-Zsanitizer=address" \
	cargo +nightly test --target x86_64-apple-darwin -p ${CRATES}
	RUSTFLAGS="-Zsanitizer=leak" \
	cargo +nightly test --target x86_64-apple-darwin -p ${CRATES}
	RUSTFLAGS="-Zsanitizer=thread" \
	cargo +nightly test --target x86_64-apple-darwin -p ${CRATES}

bench:			## Run benchmarks
	cargo bench

all-checks:			## Run all checks
all-checks: lint bench

all-tests:			## Run all tests
all-tests: loom test sanitizer

all:			## Runn all tests and checks
all: all-checks all-tests 
