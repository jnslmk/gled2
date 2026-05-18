# Rust Style Guide

## Purpose 🎯

This guide defines the Rust standard for new projects and new crates in this repository.
It is practical, opinionated, and optimized for consistency across teams.

## Core Principles 🧭

1. Prefer readable, explicit code over clever code.
2. Model domain and state in types.
3. Treat errors as part of API design.
4. Keep visibility minimal by default.
5. Include logging and tests as first-class implementation work.
6. Use async only where it adds real value.
7. Reserve panics for invariants, tests, and build scripts.

## Existing Conventions We Keep ✅

1. All dependencies are managed at workspace level.
2. Runtime crates commonly start with:

```rust
#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
```

3. Errors are modeled with `thiserror`.
4. Runtime diagnostics use `tracing`, not `println!`.
5. Public APIs are surfaced through `lib.rs`; internals stay `pub(crate)`.
6. Serde derives are standard for domain/protocol types.
7. Async stack is `tokio` (channels, tasks, timeouts).

## Tooling Rules 🔧

### Toolchain

1. Pin a Rust toolchain.
2. Set the edition explicitly and use a current stable edition.
3. Put dependencies in workspace config when shared by multiple crates.

### Formatting

1. Always format with `rustfmt`.
2. If `.rustfmt.toml` exists, it is mandatory.
3. Team default supports wide lines (`max_width = 140`).
4. Keep grouped imports (`imports_granularity = "Crate"`, `group_imports = "StdExternalCrate"`).
5. Do not keep intentionally unformatted comment blocks without strong reason.

### Lints

`clippy` must run warning-free. New code should not introduce any `clippy` warnings, and existing warnings should be treated as debt to remove rather than noise to ignore.

Minimum for new runtime crates:

```rust
#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
```

Recommended baseline:

```rust
#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![warn(clippy::panic)]
#![warn(clippy::todo)]
#![warn(missing_debug_implementations)]
```

Allowed exceptions:

1. `expect` and `panic!` in tests.
2. `expect` in build scripts where build must fail fast.
3. `unsafe` only with documented rationale, safety invariants, and review.

## Crate and Module Structure 🧱

1. One crate, one primary responsibility.
2. Separate domain logic from infrastructure.
3. Keep shared types in dedicated modules/crates.
4. Keep cross-layer protocol types out of UI/service internals.
5. `lib.rs` should define modules and re-export stable API.
6. Large implementations belong in dedicated files.
7. Use `pub(crate)` by default; expose `pub` intentionally.
8. File item order is mandatory: `mod`, then `use`, then `static`, then the rest of the code.

Recommended skeleton:

```rust
#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]

mod error;
mod service;
mod types;

pub use error::Error;
pub use service::Service;
pub use types::{Request, Response};
```

## Naming ✍️

1. Types/traits/enums: `PascalCase`.
2. Functions/methods/modules/files: `snake_case`.
3. Constants/statics: `SCREAMING_SNAKE_CASE`.
4. Prefer domain-meaningful names over technical placeholders.
5. Local abbreviations are fine (`cfg`, `ctx`, `msg`), but avoid cryptic names in public APIs.

## Type Design 🧠

1. Prefer enums for finite state spaces.
2. Prefer structs over tuples for related data.
3. Introduce newtypes when primitives have different domain meaning.
4. Avoid `String` and `serde_json::Value` as catch-all core-domain types.
5. Use `Arc<T>` only for real shared ownership or types which are heavily cloned.
6. Keep cloning intentional; cheap `Arc` clones are fine, deep clones need reason.
7. Derive traits intentionally (`Debug`, `Clone`, `Eq`, `Serialize`, etc.).
8. Do not use bitmask integers as struct fields. Represent flags as named `bool` fields or dedicated enums/sets instead.

## Functions and Methods ⚙️

1. Use precise return types.
2. Prefer strong types over raw primitives.
3. Keep parameter order consistent.
4. Avoid multiple boolean flags; use enums/options structs.
5. Keep functions focused; extract when complexity grows.
6. Prefer early returns and `let-else` to reduce nesting.
7. Use `match` for more complex branching and state handling; avoid nested `if` chains.
8. Make side effects obvious in naming (`load_...`, `publish_...`, `insert_...`).
9. Functions whose primary argument is a specific struct should be implemented as methods on that struct rather than free functions.

## Error Handling 🚨

1. Use `Result` for runtime failures.
2. Give each crate a dedicated `Error` type (typically in `error.rs`).
3. Keep errors structured and context-rich.
4. Keep public error surfaces readable; avoid leaking irrelevant internals.
5. Use serializable errors when crossing process/API boundaries.

Rules for `unwrap`, `expect`, `panic!`:

1. No `unwrap()` in production runtime code.
2. No `expect()` for recoverable production errors.
3. `panic!` only for invariants/programmer errors.
4. Panic messages must clearly state the violated assumption.

## Logging 📡

1. Use `tracing` only.
2. No `println!`/`eprintln!` in runtime code.
3. Choose log levels intentionally:
4. `trace!` for high-frequency detail.
5. `debug!` for technical flow/state.
6. `info!` for lifecycle/normal operations.
7. `warn!` for recoverable anomalies.
8. `error!` for failed operations/lost functionality.
9. Avoid log spam and never log sensitive data.

## Async and Concurrency 🔄

1. Use async for I/O/waiting APIs, not pure CPU logic.
2. Do not hide blocking work in async functions.
3. Spawn tasks sparingly with clear ownership/lifecycle.
4. Keep timeouts explicit and domain-justified.
5. Keep lock scope minimal.
6. Do not hold mutex guards across avoidable await points.
7. Prefer channel-based state flow over broad shared mutable state.
8. Keep sender/receiver lifecycle explicit.

## Serde and External Data 🌐

1. Use explicit serde attributes for protocol/persistence types.
2. Define external naming strategy intentionally (for example `camelCase`).
3. Validate external input early.
4. Do not silently coerce invalid input to defaults.
5. Use `serde_json::Value` only for truly dynamic payloads.
6. Do not expose bitmask integers in domain structs. If an external file format uses a bitmask, decode it at the boundary using `serialize_with`/`deserialize_with` and map it to typed fields (named booleans, enums, or a flags set).

## Imports 📦

1. Let `rustfmt` sort/group imports.
2. Do not order groups as std, external, internal.
3. Avoid wildcard imports except in narrow test contexts.
4. Treat long import lists as a design smell.
5. Keep re-exports in explicit API modules.

## Comments and Rustdoc 📝

Comments should explain why, not what is obvious.

Rustdoc is required for:

1. Public types.
2. Public functions with non-trivial behavior.
3. Non-obvious config/lifecycle functions.
4. Public APIs used by other crates/teams.

A good Rustdoc section answers:

1. What does it do?
2. Preconditions?
3. Side effects?
4. Error/state behavior?

## Testing 🧪

1. Unit tests near core logic.
2. Integration tests under `tests/` for boundaries.
3. Use `tokio::test` for real async paths.
4. Keep tests deterministic.
5. Test names should describe behavior.
6. Prefer one domain assertion per test.
7. Keep fixtures minimal and documented.

## Build Scripts and Macros 🏗️

1. Build scripts may fail hard when a step is mandatory.
2. Build-script failures must have concrete messages.
3. Use procedural macros only when they remove real repetition or enforce API consistency.
4. Macro errors must be understandable.

## API Design Between Crates 🔌

1. Export only intentional public API.
2. Use `lib.rs` as a designed facade.
3. Internal module structure is not API unless explicitly exported.
4. Keep public return/error types stable and readable.
5. Treat cross-crate API changes as architecture decisions.

## Preferred Idioms 👍

1. `let-else` guard clauses.
2. Small, strong domain types.
3. `thiserror` for error modeling.
4. `tracing` for diagnostics.
5. Early return over deep nesting.
6. Explicit `match` for domain states.

Avoid:

1. Oversized service files.
2. `String` as universal domain error.
3. `serde_json::Value` as typed-model replacement.
4. `unwrap`/`expect` in normal runtime code.
5. Overly broad mutable global state.

## New Project Baseline 🚀

1. Multi-crate workspace layout.
2. Shared dependencies at workspace level.
3. Repository `rustfmt` config.
4. Lints against `unsafe`, `unwrap`, and ideally `expect`.
5. `thiserror` + `tracing` by default.
6. API facade via `lib.rs`.
7. Restrictive visibility (`pub(crate)` first).
8. Unit + integration tests.
9. No panic-based normal error handling.

## PR Review Checklist ✅

1. Is ownership/responsibility clear?
2. Is visibility minimal?
3. Are names domain-precise?
4. Are errors typed and contextual?
5. Any unjustified `unwrap`/`expect`/`panic!` in runtime code?
6. Are logs level-appropriate and useful?
7. Is external data typed and validated?
8. Is async code lock-safe and non-blocking?
9. Is the public API intentionally shaped?
10. Are behavior-focused tests included?

## Quick Crate Template

```rust
#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]

mod error;
mod service;
mod types;

pub use error::Error;
pub use service::Service;
pub use types::{Request, Response};
```

Suggested layout:

```text
src/
  lib.rs
  error.rs
  service.rs
  types.rs
  domain/
  infrastructure/
  tests/
```

## Final Note

This guide is intentionally stricter than parts of the current codebase.
For new work, prefer consistency over historical exceptions.
