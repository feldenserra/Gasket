# gasket-cli

The `gasket` command for [Gasket](https://github.com/feldenserra/Gasket).

Install this package. An application depends on [`gasket-ui`](https://crates.io/crates/gasket-ui), not on `gasket-cli`.

```bash
cargo install gasket-cli
```

The binary is named `gasket`. The crates.io package is `gasket-cli` because the name `gasket` belongs to an unrelated project.

```bash
gasket dev     # server and window
gasket serve   # server only
gasket ui      # window only, using client.url from gasket.toml
```

Run it from an application directory that has `gasket.toml` and a `Cargo.toml`. From the Gasket repository it runs `examples/hello`.

`gasket new` is not built yet.

Documentation for building an app is [docs.rs/gasket-ui](https://docs.rs/gasket-ui). The guide is the [repository README](https://github.com/feldenserra/Gasket#readme).

Gasket is source-available under the [Gasket Source-Available License 1.0](https://github.com/feldenserra/Gasket/blob/main/LICENSE).
