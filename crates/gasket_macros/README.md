# gasket-macros

Compile-time discovery of `src/api` and `src/ui` for [Gasket](https://github.com/feldenserra/Gasket).

Applications depend on [`gasket-ui`](https://crates.io/crates/gasket-ui). `gasket_ui::app!` calls this crate. Application code does not.

[docs.rs/gasket-ui](https://docs.rs/gasket-ui) · [Repository](https://github.com/feldenserra/Gasket)

## What this crate does

This is a procedural macro. At compile time it reads the application crate's `src/api` and `src/ui` folders and expands `gasket_ui::app!` into modules, routes, and `fn main`.

The `gasket` command is [`gasket-cli`](https://crates.io/crates/gasket-cli).

Gasket is source-available under the [Gasket Source-Available License 1.0](https://github.com/feldenserra/Gasket/blob/main/LICENSE).
