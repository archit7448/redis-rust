# redis-rust

A small Redis-style server written in Rust.

## What it does

- Listens on `127.0.0.1:6379`
- Parses basic RESP messages
- Supports `SET` and `GET`
- Stores data in an in-memory `HashMap`

## Run

```bash
cargo run
```

## Example

Use `redis-cli` against the local server:

```bash
redis-cli -p 6379 SET foo bar
redis-cli -p 6379 GET foo
```
