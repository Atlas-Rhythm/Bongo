# Bongo

**WIP**

Mongoose meets Rust

## About

Bongo is a MongoDB ODM for Rust, which provides wrappers around the [official driver](https://github.com/mongodb/mongo-rust-driver). It's centered around the `BlockingModel` and `Model` traits, which can easily be implemented on structs using procedural macros. `BlockingModel` provides a synchronous interface very similar to the one of Mongoose, while `Model` wraps synchronous calls inside asynchronous functions using Tokio.

## Why

Cutting out the boilerplate code required for converting to and from BSON, running queries from async contexts, creating indexes and just making everything cleaner and easier in general.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).
