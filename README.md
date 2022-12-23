# Fremkit

![Crates.io](https://img.shields.io/crates/v/fremkit) ![docs.rs](https://img.shields.io/docsrs/fremkit)

Fremkit is a simple broadcast log.

A Log's primary use case is to store an immutable sequence of messages, events, or other data, and to allow multiple readers to access the data concurrently.

## Features

- Bounded log structure with fixed size.
- Fast and efficient, with performances comparable to a `Mutex<Vec<_>>`. (See benchmarks)
- Lock-free, and thread-safe design.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
fremkit = "^0.1"
```

## Example

```rust
use fremkit::bounded::Log;

let log: Log<u64> = Log::new(100);
log.push(1).unwrap();
log.push(2).unwrap();

assert_eq!(log.get(0), Some(&1));
assert_eq!(log.get(1), Some(&2));
```

## License

This crate is under the Apache-2.0 License.
