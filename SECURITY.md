# Security Policy

Gasket is an API-first native framework: `gasket_core` accepts HTTP and WebSocket traffic, `gasket_protocol` is the stream contract, `gasket_ui` renders untrusted application markup, and `gasket_cli` executes local build and scaffolding commands. Vulnerabilities in any of those surfaces can affect host machines and downstream applications. Please report them privately.

---

## Supported versions

Gasket is in early development. Security fixes are applied on the default branch (`master`) and included in the next tagged release.

| Version | Supported |
| --- | --- |
| `master` (unreleased 0.x) | Yes |
| Latest tagged 0.x release | Yes, until the following tag |
| Older 0.x tags | No |

Once 1.x is released, this table will list the maintained minor lines. Until then, assume only current `master` and the newest tag receive patches.

---

## Reporting a vulnerability

**Do not** file a public GitHub issue, discussion, or pull request for a suspected vulnerability.

Report privately through GitHub Security Advisories:

**[https://github.com/feldenserra/Gasket/security/advisories/new](https://github.com/feldenserra/Gasket/security/advisories/new)**

If you cannot use GitHub advisories, open a private contact request against the repository at [https://github.com/feldenserra/Gasket](https://github.com/feldenserra/Gasket) **without** including exploit details in a public channel, and ask for a secure follow-up path.

### What to include

- Affected crate (`gasket_cli`, `gasket_core`, `gasket_protocol`, `gasket_ui`, stitch toolchain) and git commit or version
- Description of the issue and its security impact (crash, sandbox escape, RCE, data leak, privilege escalation, protocol spoofing, etc.)
- Reproduction steps or a minimal proof-of-concept you are willing to share privately
- Any known in-the-wild exploitation
- Your preferred credit name or a request to remain anonymous

Do not attach credentials, customer data, or production logs that are not required to reproduce the issue.

---

## Response process

We will make a good-faith effort to meet the following timelines:

| Stage | Target |
| --- | --- |
| Acknowledgement | Within **3 business days** |
| Initial triage (severity, affected surface, reproducibility) | Within **10 business days** of a complete report |
| Fix or coordinated disclosure plan for confirmed issues | As soon as practical; we will keep you informed if the work extends beyond **90 days** |

You can expect:

1. Confirmation that the report was received.
2. A decision: confirmed, duplicate, informational, or not a vulnerability.
3. A private discussion of severity, affected versions, and disclosure date.
4. Credit in the advisory unless you decline it.

We use qualitative severity (low / moderate / high / critical) based on impact and exploitability. There is no bug bounty program at this time.

---

## Disclosure

Please allow us to patch and publish an advisory before you discuss the issue in public. We prefer coordinated disclosure:

- We will credit reporters in the GitHub Security Advisory and release notes unless asked otherwise.
- We may share the report with downstream maintainers if a third-party dependency is the root cause.
- We will not ask you to withhold a report indefinitely. If we fail to respond within the acknowledgement window, you may follow up; if we still do not respond, public disclosure after **90 days** from the initial report is reasonable.

---

## Scope

**In scope**

- Remote or local code execution via `gasket_core` HTTP/WebSocket handling
- Privilege escalation, path traversal, or unexpected filesystem writes from `gasket_cli`
- Injection, sandbox escape, or unsafe rendering from stitch / `gasket_ui`
- Authentication, authorization, or session-fixation flaws in the core protocol
- Secret leakage in logs, error messages, or packaged binaries
- Supply-chain issues in this repository (compromised release artifacts, malicious CI, unexpected `build.rs` network activity)

**Out of scope** (unless you can show a Gasket-specific exploit path)

- Vulnerabilities only in unmodified upstream dependencies (report those upstream; tell us if Gasket needs a version bump)
- Denial of service that requires already-local, already-privileged access
- Missing hardening that has no demonstrated impact
- Social engineering, physical access, or issues in applications *built with* Gasket that do not stem from the framework itself

---

## Security hygiene for contributors

- Never commit secrets, tokens, or private keys. Rotate anything that was committed and treat it as compromised.
- Prefer safe defaults in protocol and renderer code: explicit allow-lists, bounded allocations, and no `unsafe` without a documented invariant.
- New network, IPC, or markup-parsing surfaces should include tests for malformed input.

Questions about this policy that do not include a vulnerability report can be asked in a public issue. Everything that might be exploitable belongs in a private advisory.
