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
│              dev runs an application binary                 │
└──────────────────────────┬──────────────────────────────────┘
                           │ cargo run
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     application binary                      │
│   src/main.rs is gasket_ui::app!()                          │
│   src/api/<name>   get() -> Response                        │
│   src/ui/<name>    show(ui, data, nav)                      │
└───────────────┬─────────────────────────────┬───────────────┘
                │                             │
                ▼                             ▼
       ┌─────────────────┐           ┌─────────────────┐
       │   gasket-core   │  HTTP     │    gasket-ui    │
       │   Axum routes   │◀─────────▶│  native shell   │
       │                 │  /ws      │  (egui for now) │
       └─────────────────┘           └─────────────────┘
```

- **`gasket-cli`** — The `gasket` command. It runs an application. It does not contain one. The crates.io package is `gasket-cli` because the name `gasket` is already taken.
- **`gasket-core`** — The Axum engine. It serves the router the shell built.
- **`gasket-ui`** — Starts that server, opens the window, and feeds each page. Today the page UI is throwaway egui. The destination is a GPU renderer that compiles **stitch** into a retained scene graph.
- **`gasket-macros`** — Reads `src/api` and `src/ui` at compile time so `main.rs` stays one line. Authors do not call it.

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
examples/hello/src/main.rs             gasket_ui::app!();
examples/hello/src/api/index/mod.rs    dashboard JSON
examples/hello/src/ui/index/mod.rs     home page, buttons open other pages
examples/hello/src/api/hello/mod.rs    owns Wish, read with req.json
examples/hello/src/ui/hello/mod.rs     imports Wish once, nav.submit
examples/hello/src/api/about/mod.rs    owns Note, read with req.json
examples/hello/src/ui/about/mod.rs     imports Note once, nav.submit
examples/hello/src/api/stream/mod.rs   STREAM_EVERY of 1 second
examples/hello/src/ui/stream/mod.rs    draws that response
examples/hello/src/api/pulse/mod.rs    STREAM_EVERY of 5 seconds
examples/hello/src/ui/pulse/mod.rs     draws that response
examples/hello/src/api/health/mod.rs   a route with no page
```

The same app directly:

```bash
cargo run -p gasket-hello
```

`gasket new` is not built yet. When it exists, a new application will be this same layout.

---

## How an application is shaped

`src/main.rs` is only `gasket_ui::app!()`. Cargo needs the folders under `src/`.

| Folder | Required items | What the shell does |
| --- | --- | --- |
| `src/api/<name>/mod.rs` | `get(req) -> Response`, optional `post(req)` | `GET` and, when `post` exists, `POST /api/<name>` |
| `src/ui/<name>/mod.rs` | `show(ui, data, nav)` | Draws that `Response` |
| `src/api/<name>/mod.rs` with `STREAM_EVERY` | the same `get`, plus the const | While that page is open, one `/ws` subscription pushes `get()` at that interval |
| `src/api/<name>/mod.rs` and no `src/ui/<name>` | `get()` only | The route is callable. It has no page. |

A page with no matching route does not compile. Folder names are Rust identifiers (`hello`, not `my-page`).

`index` is home. The window opens there. Its buttons are data: `nav.open("hello")` uses the page name from `GET /api/index`. Back, shown by the shell on every other page, returns to `index`.

```rust
pub const STREAM_EVERY: std::time::Duration = std::time::Duration::from_secs(5);
```

That const is the whole streaming switch. It lives on the route. The page does not mention sockets. `get` and `post` receive a `gasket_ui::Request` with method, path, query, headers, and body. The shell keeps one WebSocket, for the page on screen, and closes it when you leave. The server stops calling `get()` when that socket closes. Opening the page again calls `get()` once for the first paint, then on each interval.

`gasket.toml` next to the app says where to listen and where the window dials:

```toml
[server]
bind = "127.0.0.1"
port = 3000

[client]
url = "http://127.0.0.1:3000"
```

A host sets `bind` to `0.0.0.0` and `url` to the public `http://` address. TLS stays in front of the process.

```bash
gasket dev     # server and window
gasket serve   # server only
gasket ui      # window only, using client.url
```

The app binary accepts the same split as `--serve` and `--ui`. With neither flag it runs both. The file is read from the working directory, then from beside the executable, then from the copy compiled into the app.

`get()` should return quickly. A counter in the reference app moves only when `get()` runs. It is not a clock, and it does not reset when you leave.

A page's `State` does reset when you leave that page and open it again. The shell does that. Refresh and Submit keep what you typed.

The reference app:

| Page | Route | Transport |
| --- | --- | --- |
| Index | `GET /api/index` | One HTTP response. Buttons open the other pages. |
| Hello | `GET` / `POST /api/hello` | The route owns `Wish`. The page imports it once and calls `nav.submit`. The route reads `req.json::<Wish>`. |
| About | `GET` / `POST /api/about` | GET still reads `?name=`. The route owns `Note`. The page imports it once and calls the same `nav.submit`. |
| Stream | `GET /api/stream` | `STREAM_EVERY` is 1 second, pushed on `/ws` while the page is open. |
| Pulse | `GET /api/pulse` | `STREAM_EVERY` is 5 seconds, pushed on `/ws` while the page is open. |
| — | `GET /api/health` | No page. |

```bash
curl http://127.0.0.1:3000/api/hello
curl http://127.0.0.1:3000/api/health
```

---

## Repository layout

```text
gasket/
├── crates/
│   ├── gasket_cli/          # `gasket` binary, package `gasket-cli`
│   ├── gasket_core/         # Axum / WebSocket engine, package `gasket-core`
│   ├── gasket_macros/       # Folder discovery, package `gasket-macros`
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
