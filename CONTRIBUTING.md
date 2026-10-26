# Contributing to Gasket

Thank you for considering a contribution to Gasket. This document is the working contract for how we take changes: how to report issues, how to open pull requests, and the Rust standards every change is expected to meet.

By contributing, you agree that your work is licensed to the project under the terms in [LICENSE](./LICENSE), including the right to offer that work under the Gasket Source-Available License and under commercial licenses.

---

## Where to start

1. Search [existing issues](https://github.com/feldenserra/Gasket/issues) and pull requests before opening a new one.
2. For a substantial change (new crate surface, stitch syntax, CLI commands, protocol changes), open an issue first so design can land before code.
3. Small, well-scoped fixes — compiler warnings, docs, tests, obvious bugs — may go straight to a pull request.

Please do **not** use the public issue tracker for security vulnerabilities. Follow [SECURITY.md](./SECURITY.md).

---

## Issue reports

Use a dedicated issue per problem or proposal. Title it so a stranger can understand the report without opening it.

### Bug report

Include:

- Gasket version or git commit SHA
- Host OS, architecture, and Rust toolchain (`rustc --version`)
- Minimal reproduction: commands, stitch snippet, or crate-level example
- Expected behavior vs. actual behavior
- Full error output, panic backtrace, or relevant logs (fenced in code blocks)

### Feature or design proposal

Include:

- The problem, not only the solution
- Who it is for (CLI users, core API authors, stitch UI authors)
- Proposed shape of the API, command, or markup
- Alternatives considered and why they were rejected

### Issue templates (when filing on GitHub)

Until GitHub issue forms ship in `.github/ISSUE_TEMPLATE/`, structure the body with these headings:

```markdown
## Summary

## Reproduction (bugs) / Motivation (proposals)

## Expected behavior

## Actual behavior / Proposed behavior

## Environment

- OS:
- Rustc:
- Gasket commit:
```

---

## Development setup

Gasket is a Cargo workspace. Use a current stable Rust toolchain.

```bash
git clone https://github.com/feldenserra/Gasket.git
cd Gasket
rustup show
cargo build --workspace
cargo test --workspace
```

Useful commands:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

Do not commit `target/` or `Cargo.lock` changes unless a workspace-level lockfile policy is explicitly adopted.

---

## Pull requests

### Branching

- Fork the repository and create a topic branch from `master`.
- Name branches by intent: `fix/window-focus`, `feat/stitch-for-loop`, `docs/quickstart`.
- Keep the branch focused. Unrelated refactors belong in a separate PR.

### Scope

A pull request should do one thing. Mix of formatting-only churn and behavioral changes is rejected. Match existing crate structure, naming, error handling, and dependency injection; do not introduce new libraries or architectural layers unless the change is impossible with the current plumbing.

### Description

The PR body should explain **why** the change exists, not only what files moved. Include:

- Linked issue (`Fixes #123`) when applicable
- Summary of user-visible behavior
- Test plan: commands you ran and cases you covered
- Breaking changes, CLI flag changes, or stitch syntax changes, called out explicitly

### Review

- Keep commits readable. Prefer a short series of logical commits over one opaque dump.
- Respond to review comments on the thread, even when you disagree.
- Do not force-push over commits another reviewer is mid-reviewing unless asked.
- Maintainers may request squash-on-merge; you do not need to squash locally unless the history is noisy.

### Checklist (author)

Before requesting review:

- [ ] `cargo fmt --all` has been applied
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean
- [ ] `cargo test --workspace` passes
- [ ] Public items have rustdoc where they form a crate contract
- [ ] No secrets, credentials, or machine-local paths are included
- [ ] LICENSE, copyright headers, and attribution are unchanged unless the PR is specifically about licensing

---

## Rust code style

Gasket follows idiomatic Rust as encoded by **rustfmt** and **Clippy**. Do not fight the formatter.

| Rule | Expectation |
| --- | --- |
| Formatter | rustfmt defaults (`cargo fmt --all`). No `#[rustfmt::skip]` unless generated code or a documented exception. |
| Lints | Clippy pedantic enough that `-- -D warnings` stays clean. Fix warnings; do not `#[allow]` without a one-line justification. |
| Edition | Rust 2021, matching the workspace crates. |
| Naming | `snake_case` functions and modules, `CamelCase` types, `SCREAMING_SNAKE_CASE` constants. Crate names stay `gasket_*`. |
| Errors | Prefer structured error types (`thiserror` / `anyhow` only where the crate already uses them). Do not `unwrap()` in library paths; `expect` only with a comment stating the invariant. |
| Async | Use Tokio. Do not block the runtime (`std::thread::sleep`, heavy `std::fs` in async context) without an explicit offload. |
| Traits & APIs | Keep public surface small. New public items need rustdoc. Avoid `pub use` glob re-exports unless they already exist as a crate facade. |
| Unsafe | Last resort. Every `unsafe` block has a `# Safety` comment stating the invariants you are upholding. |
| Tests | Unit tests live next to the code (`#[cfg(test)]`). Behavior that crosses crates belongs in integration tests under `tests/`. |
| Dependencies | Reuse versions already in the workspace. New dependencies require a justification in the PR (size, license, maintenance). |

Module layout should follow the crate you are editing. `gasket_core` is the Axum/WebSocket engine; `gasket_ui` is the native renderer; `gasket_cli` is the developer tool. Put code in the crate that owns that responsibility.

---

## Commit messages

Write in the imperative mood, with a short subject and an optional body for motivation:

```text
Add WebSocket heartbeat to gasket_core

Idle clients were dropped by reverse proxies after 60s. Send a
protocol-level ping so long-lived UI sessions stay connected.
```

Prefix with `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, or `chore:` when the change maps cleanly to one of those. The subject should be accurate if it appears in a changelog.

---

## Code of interaction

Be precise and civil. Critique the design, not the person. Maintainers will close threads that are hostile, off-topic, or repeatedly ignore this guide.

---

## License of contributions

Gasket is dual-licensed: source-available under [LICENSE](./LICENSE), with commercial licenses sold to enterprises and managed-service operators.

Submitting a contribution (code, tests, documentation, examples, or review comments that include patches) means you:

1. Certify that you have the right to submit the work (you wrote it, or it is under a compatible license you can relicense).
2. Grant the Gasket authors the rights described in Section 6 of the LICENSE, including relicensing under commercial terms.

If your employer has rights over your work, obtain permission before submitting.
