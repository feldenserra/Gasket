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

Gasket is a **unified monorepo**. Three crates form the framework; **stitch** is the markup language they share.

```text
┌─────────────────────────────────────────────────────────────┐
│                        gasket_cli                           │
│         create · dev · build · package · inspect            │
└──────────────────────────┬──────────────────────────────────┘
                           │
           ┌───────────────┴───────────────┐
           ▼                               ▼
┌─────────────────────┐         ┌─────────────────────────┐
│    gasket_core      │  WS /   │       gasket_ui         │
│  Axum HTTP +        │  state  │  GPU-accelerated native │
│  WebSocket engine   │ streams │  renderer (stitch)      │
└─────────────────────┘         └─────────────────────────┘
```

- **`gasket_cli`** — The developer surface. Scaffolds projects, runs the unified development loop, and produces release binaries. Distributed as the `gasket` command.
- **`gasket_core`** — The backend engine. An [Axum](https://github.com/tokio-rs/axum) HTTP and WebSocket runtime that owns application APIs, sessions, and real-time state replication. The UI is a client of this API, not a second source of truth.
- **`gasket_ui`** — The native client renderer. A GPU-accelerated process that compiles **stitch** markup into retained native scene graphs. No HTML parser. No DOM. Layout and paint stay in Rust.

**stitch** is the UI markup language: declarative, component-oriented, and designed to bind directly against core-streamed state. You write structure and intent; `gasket_ui` handles rendering.

---

## Quickstart

The CLI is the intended entry point. Once published, a new application looks like this:

```bash
# Install the Gasket CLI
cargo install gasket

# Scaffold a native desktop app
gasket new my-app
cd my-app

# Run the unified dev loop (core + native UI, live reload)
gasket dev
```

`gasket new` will generate a workspace with a `gasket_core` API module, a `gasket_ui` stitch tree, and a local configuration file. `gasket dev` will start the Axum engine, open the native renderer, and stream state between them over WebSocket.

Build a release binary with:

```bash
gasket build --release
```

> The CLI surface is coming online. Until `gasket new` ships, clone this repository and use the workspace crates directly:

```bash
git clone https://github.com/feldenserra/Gasket.git
cd Gasket
cargo build --workspace
```

---

## Repository layout

```text
gasket/
├── crates/
│   ├── gasket_cli/     # Developer CLI (`gasket`)
│   ├── gasket_core/    # Axum / WebSocket backend engine
│   └── gasket_ui/      # GPU-accelerated native renderer
├── LICENSE
├── CONTRIBUTING.md
├── SECURITY.md
└── README.md
```

---

## Licensing

Gasket is **source-available** under the [Gasket Source-Available License 1.0](./LICENSE). It is free for indie developers and small teams; it is not an OSI-approved open-source license, and it does **not** convert to MIT/Apache/GPL after a waiting period.

| Who | What you may do |
| --- | --- |
| Individuals, hobbyists, students, and researchers | Use, modify, and build with Gasket at no charge |
| Internal development teams and commercial entities with **≤ USD $5,000,000** annual gross revenue | Production use at no charge, provided you are not offering Gasket as a paid managed service |
| Enterprises, corporations, or commercial entities with **> USD $5,000,000** annual gross revenue | A **commercial license is required** |
| Anyone offering Gasket as a **paid managed service** | A **commercial license is required**, regardless of revenue |

The revenue threshold and managed-service restriction are **permanent**.

Commercial licensing inquiries: [github.com/feldenserra/Gasket](https://github.com/feldenserra/Gasket)

---

## Contributing

We welcome issue reports, design discussion, and pull requests. Please read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening a PR, and [SECURITY.md](./SECURITY.md) before reporting a vulnerability.

---

## Status

Gasket is in early development. APIs, stitch syntax, and CLI commands will change as the framework takes shape. Star the repository and watch releases if you want to follow along.
