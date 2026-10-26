# gasket-core

HTTP and WebSocket engine for [Gasket](https://github.com/feldenserra/Gasket).

Applications depend on [`gasket-ui`](https://crates.io/crates/gasket-ui). That crate depends on this one. Do not add `gasket-core` to an application `Cargo.toml`.

[docs.rs/gasket-ui](https://docs.rs/gasket-ui) · [Repository](https://github.com/feldenserra/Gasket)

## What this crate does

`gasket-core` binds a listener and serves the Axum router that `gasket-ui` builds. Application routes live in the app, under `src/api`. This crate does not discover them.

The `gasket` command is [`gasket-cli`](https://crates.io/crates/gasket-cli).

Gasket is source-available under the [Gasket Source-Available License 1.0](https://github.com/feldenserra/Gasket/blob/main/LICENSE).
