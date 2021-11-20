SHELL := /bin/bash
.PHONY: help lint loom test sanitizer bench

CRATES = fremkit-channel \
		 fremkit-maker

help:			## Show this help
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z0-9_-]+:.*?## / {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

lint:			## Run clippy
	cargo clippy

loom:			## Run tests with loom
	RUSTFLAGS="--cfg loom" \
	LOOM_MAX_PREEMPTIONS=2 \
	cargo test --release $(addprefix -p, ${CRATES})

test:			## Run tests
	cargo test

sanitizer:		## Run tests with sanitizer
	RUSTFLAGS="-Zsanitizer=address" \
	cargo +nightly test --target x86_64-apple-darwin $(addprefix -p, ${CRATES})
	RUSTFLAGS="-Zsanitizer=leak" \
	cargo +nightly test --target x86_64-apple-darwin $(addprefix -p, ${CRATES})
	RUSTFLAGS="-Zsanitizer=thread" \
	cargo +nightly test --target x86_64-apple-darwin $(addprefix -p, ${CRATES})

bench:			## Run benchmarks
	@mv dist/benchmark target/criterion 2> /dev/null || true
	cargo bench $(addprefix -p, ${CRATES})
	mv target/criterion dist/benchmark

all:			## Runn all tests and checks
all: lint loom test sanitizer bench
