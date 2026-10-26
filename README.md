# Gasket 🚀

**The API-First, High-Performance Native Framework for Rust (An Electron/Next.js Alternative).**

Gasket is a unified framework for building native desktop applications in Rust. You describe interfaces in **stitch**, stream application state over a first-class API, and ship a single compiled binary — no Chromium, no WebKit, no DOM.

> Desktop apps with the developer experience of a modern full-stack framework, and the runtime profile of native Rust.

---

## Why Gasket

Traditional desktop stacks inherited the web browser. Electron embeds Chromium. Many "native" toolkits still parse HTML, walk a DOM, and pay JavaScript-bridge costs on every interaction. That model is familiar. It is also heavy, slow to start, and difficult to secure.

Gasket takes a different path:

| Browser-centric stacks | Gasket |
| --- | --- |
| Ship a full browser engine with every app | Compile a native Rust binary |
| Layout through HTML/CSS and DOM mutation | Render **stitch** markup on the GPU |
| HTTP request/response plus ad-hoc IPC | Real-time state streaming from day one |
| Split frontend, backend, and packaging toolchains | One monorepo, one CLI, one mental model |

The result is an API-first architecture: your application logic lives in Rust, your UI is a declarative view over streamed state, and the renderer never bootstraps a browser.

---

## Architecture

Gasket is a **unified monorepo**. The framework crates plus **stitch** are the product. An application is folders, not startup code.

```text
┌─────────────────────────────────────────────────────────────┐
│                        gasket-cli                           │
│         create · dev · build · package · inspect            │
└──────────────────────────┬──────────────────────────────────┘
                           │ orchestrates an application
           ┌───────────────┴───────────────┐
           ▼                               ▼
┌─────────────────────┐    gasket-protocol ┌─────────────────────────┐
│    gasket-core      │◀──────────────────▶│       gasket-ui         │
│  Axum HTTP +        │   state stream     │  native shell           │
│  WebSocket engine   │      (/ws)         │  (stitch, later)        │
└─────────────────────┘                    └─────────────────────────┘
```

- **`gasket-cli`** — The developer surface, installed as the `gasket` command. It runs an application. It does not contain one. The crates.io package is `gasket-cli` because the name `gasket` is already taken.
- **`gasket-core`** — The backend engine. An [Axum](https://github.com/tokio-rs/axum) HTTP and WebSocket runtime. It serves the routes an application declares and reserves `/ws`.
- **`gasket-protocol`** — The internal wire contract between the engine and the shell. A page's model is the `Response` struct in that route, not a type added here.
- **`gasket-ui`** — The native shell. It opens the window and loads each page from its route. Today the page UI is throwaway egui. The destination is a GPU renderer that compiles **stitch** into a retained scene graph: no HTML parser, no DOM.
- **`gasket-macros`** — Finds `src/api` and `src/ui` folders so `main.rs` stays one line. Authors do not call it directly.

**stitch** is the UI markup language: declarative, component-oriented, and designed to bind directly against core-streamed state. You write structure and intent; `gasket_ui` handles rendering. The current shell is not that renderer.

---

## Quickstart

Clone the repository and start the reference app:

```bash
git clone https://github.com/feldenserra/Gasket.git
cd Gasket
cargo run -p gasket-cli -- dev
```

`gasket dev` runs `examples/hello`. That app is the developer experience:

```text
examples/hello/src/main.rs            gasket_ui::app!();
examples/hello/src/api/hello/mod.rs   get() -> Response
examples/hello/src/ui/hello/mod.rs    show(ui, data)
```

`src/api/hello` is `GET /api/hello`. `src/ui/hello` draws that response. Adding a feature means adding a folder. The framework starts the server and the window.

```bash
curl http://127.0.0.1:3000/api/hello
```

The same app directly:

```bash
cargo run -p gasket-hello
```

`gasket new` is not built yet. When it exists, a new application will be this same layout: one `main.rs`, an `api` folder per route, and a `ui` folder per page.

---

## Repository layout

```text
gasket/
├── crates/
│   ├── gasket_cli/          # `gasket` binary, package `gasket-cli`
│   ├── gasket_core/         # Axum / WebSocket engine, package `gasket-core`
│   ├── gasket_macros/       # Folder discovery, package `gasket-macros`
│   ├── gasket_protocol/     # Internal wire messages, package `gasket-protocol`
│   └── gasket_ui/           # Native shell, package `gasket-ui`
├── examples/
│   └── hello/               # Reference app: src/main.rs, src/api, src/ui
├── LICENSE
├── CONTRIBUTING.md
├── SECURITY.md
└── README.md
```

Rust import paths use underscores (`gasket_core`). Package names use hyphens (`gasket-core`).

---

## Licensing

Gasket is **source-available** under the [Gasket Source-Available License 1.0](./LICENSE). It is free for indie developers and small teams; it is not an OSI-approved open-source license, and it does **not** convert to MIT/Apache/GPL after a waiting period.

| Who | What you may do |
| --- | --- |
| Individuals, hobbyists, students, and researchers | Use, modify, and build with Gasket at no charge |
| Internal development teams and commercial entities with **≤ USD $1,000,000** annual gross revenue | Production use at no charge, provided you are not offering Gasket as a paid managed service |
| Enterprises, corporations, or commercial entities with **> USD $1,000,000** annual gross revenue | A **commercial license is required** |
| Anyone offering Gasket as a **paid managed service** | A **commercial license is required**, regardless of revenue |

The revenue threshold and managed-service restriction are **permanent**.

Commercial licensing inquiries: [github.com/feldenserra/Gasket](https://github.com/feldenserra/Gasket)

---

## Contributing

We welcome issue reports, design discussion, and pull requests. Please read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening a PR, and [SECURITY.md](./SECURITY.md) before reporting a vulnerability.

---

## Status

Gasket is in early development. APIs, stitch syntax, and CLI commands will change as the framework takes shape. Star the repository and watch releases if you want to follow along.
