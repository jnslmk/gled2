# Rust Style Guide

## Purpose 🎯

This guide defines the Rust standard for this binary application codebase.
It is practical, opinionated, and optimized for consistency across the team.

## Core Principles 🧭

1. Prefer readable, explicit code over clever code.
2. Model domain and state in types.
3. Treat errors as recoverable runtime conditions, not exceptional edge cases.
4. Keep visibility minimal by default — there is no external consumer.
5. Include logging and tests as first-class implementation work.
6. Use async only where it adds real value.
7. Reserve panics for invariants, tests, and build scripts.

## Existing Conventions We Keep ✅

1. All dependencies are managed at workspace level.
2. `main.rs` starts with:

```rust
#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(missing_debug_implementations)]
```

3. Errors are modeled with `anyhow`.
4. Runtime diagnostics use `tracing`, not `println!`.
5. Internal modules use `pub(crate)` by default; `pub` is only used where cross-module access is genuinely needed.
6. Serde derives are standard for domain/protocol types.
7. Channel stack is `kanal`; async stack is `tokio` where needed.

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

`clippy` must run warning-free. New code must not introduce any `clippy` warnings, and existing warnings are debt to remove rather than noise to ignore.

Required baseline in `main.rs`:

```rust
#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(missing_debug_implementations)]
```

Allowed exceptions:

1. `expect` and `panic!` in tests.
2. `expect` in build scripts where build must fail fast.
3. `expect` for true programmer invariants — the message must clearly state what assumption was violated.
4. `unsafe` only with documented rationale, safety invariants, and review.

## Module Structure 🧱

1. `main.rs` is the entry point and module root. Keep it thin — startup, wiring, and `fn main`.
2. Declare all top-level modules in `main.rs` using `pub(crate) mod`.
3. Separate domain logic from infrastructure and UI.
4. Keep shared types in dedicated modules rather than duplicating them.
5. Large implementations belong in dedicated files, not sprawling `mod.rs` files.
6. Use `pub(crate)` by default. Only use `pub` when a type or function is intentionally part of a cross-module API.
7. File item order is mandatory: `mod`, then `use`, then `static`/`const`, then the rest of the code.

Recommended `main.rs` skeleton:

```rust
#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(missing_debug_implementations)]

mod audio;
mod input;
mod pipeline;
mod storage;
mod ui;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    // startup and wiring
    Ok(())
}
```

## Naming ✍️

1. Types/traits/enums: `PascalCase`.
2. Functions/methods/modules/files: `snake_case`.
3. Constants/statics: `SCREAMING_SNAKE_CASE`.
4. Prefer domain-meaningful names over technical placeholders.
5. Local abbreviations are fine (`cfg`, `ctx`, `msg`), but avoid cryptic names at module boundaries.

## Type Design 🧠

1. Prefer enums for finite state spaces.
2. Prefer structs over tuples for related data.
3. Introduce newtypes when primitives have different domain meaning.
4. Avoid `String` and `serde_json::Value` as catch-all core-domain types.
5. Use `Arc<T>` only for real shared ownership or types that are heavily cloned across threads.
6. Keep cloning intentional; cheap `Arc` clones are fine, deep clones need reason.
7. Derive traits intentionally (`Debug`, `Clone`, `Eq`, `Serialize`, etc.).
8. Do not use bitmask integers as struct fields. Represent flags as named `bool` fields or dedicated enums/sets instead.

## Functions and Methods ⚙️

1. Use precise return types.
2. Prefer strong types over raw primitives.
3. Keep parameter order consistent.
4. Avoid multiple boolean flags; use enums or option structs.
5. Keep functions focused; extract when complexity grows.
6. Prefer early returns and `let-else` to reduce nesting.
7. Use `match` for more complex branching and state handling; avoid nested `if` chains.
8. Make side effects obvious in naming (`load_...`, `publish_...`, `insert_...`).
9. Functions whose primary argument is a specific struct should be implemented as methods on that struct rather than free functions.

## Error Handling 🚨

1. Use `Result` for all runtime failures, including in `fn main`.
2. Use `anyhow::Result` and `anyhow::Context` throughout — add context at every boundary.
3. Keep errors structured and context-rich; error messages read as a chain of what failed and why.
4. Do not use `expect` for recoverable errors. `expect` is only justified when the alternative is a broken invariant that the program cannot meaningfully recover from, and the message must state the assumption.

Rules for `unwrap`, `expect`, `panic!`:

1. No `unwrap()` anywhere in production runtime code.
2. `expect` only for true programmer invariants, never for I/O, parsing, or external state.
3. `panic!` only for invariants that cannot be expressed in the type system.
4. Panic messages must clearly state the violated assumption.

## Logging 📡

1. Use `tracing` only.
2. No `println!`/`eprintln!` in runtime code. The only valid use of `println!` is for intentional CLI output (license text, help output) in a dedicated code path before the tracing subscriber is initialized.
3. Choose log levels intentionally:
   - `trace!` for high-frequency detail.
   - `debug!` for technical flow/state.
   - `info!` for lifecycle/normal operations.
   - `warn!` for recoverable anomalies.
   - `error!` for failed operations/lost functionality.
4. Avoid log spam and never log sensitive data.

## Channels and Concurrency 🔄

1. Prefer `kanal` channels for all inter-thread and intra-process communication.
2. Do not use `Mutex` where a channel would suffice. Mutex is appropriate for short-lived global registration (e.g. a list of subscriber senders) and nothing more.
3. Do not hold mutex guards across any non-trivial logic, and never across await points.
4. Spawn threads and tasks sparingly with clear ownership and shutdown semantics.
5. Keep timeouts explicit and domain-justified.
6. Prefer channel-based state flow over shared mutable state.
7. Keep sender/receiver lifecycle explicit — dropped senders/receivers must be handled.
8. Use async for I/O and waiting; do not hide blocking work inside async functions.

## Serde and External Data 🌐

1. Use explicit serde attributes for all persistence and protocol types.
2. Define external naming strategy intentionally (for example `#[serde(rename_all = "camelCase")]`).
3. Validate external input early, at the deserialization boundary.
4. Do not silently coerce invalid input to defaults.
5. Use `serde_json::Value` only for truly dynamic payloads.
6. Do not expose bitmask integers in domain structs. If an external file format uses a bitmask, decode it at the boundary using `serialize_with`/`deserialize_with` and map it to typed fields (named booleans, enums, or a flags set).

## Imports 📦

1. Let `rustfmt` sort/group imports.
2. Avoid wildcard imports in production code. `use super::*` is acceptable in narrow test modules. `use wgpu::*` and similar large external wildcards should be replaced with explicit imports.
3. Treat long import lists as a design smell.

## Comments 📝

Comments should explain why, not what the code obviously does.

Document with a comment any of the following:

1. Non-obvious invariants and assumptions (`// SAFETY:`, `// INVARIANT:`).
2. Domain rules encoded in logic that aren't obvious from the code.
3. Why a particular approach was chosen over an obvious alternative.
4. Lifecycle or startup-ordering constraints.

## Testing 🧪

1. Unit tests near core logic, in a `#[cfg(test)]` block in the same file.
2. Integration tests under `tests/` for system-level behaviour.
3. Use `tokio::test` for real async paths.
4. Keep tests deterministic.
5. Test names should describe behaviour, not implementation.
6. Prefer one domain assertion per test.
7. Keep fixtures minimal and documented.

## Build Scripts and Macros 🏗️

1. Build scripts may fail hard when a step is mandatory.
2. Build-script failures must have concrete messages.
3. Use procedural macros only when they remove real repetition or enforce API consistency.
4. Macro errors must be understandable.

## Module Boundaries 🔌

There is no external API consumer for a binary, but module boundaries still matter for maintainability:

1. Each module owns a single responsibility. If a module's purpose is hard to state in one sentence, split it.
2. Avoid reaching into another module's internals. Cross-module access goes through the module's own public types and functions.
3. Keep cross-module types in a dedicated `types` or domain module, not buried inside a feature module.
4. Treat significant refactors of module boundaries as architecture decisions requiring team discussion.

## Preferred Idioms 👍

1. `let-else` guard clauses.
2. Small, strong domain types.
3. `anyhow` for error propagation.
4. `tracing` for diagnostics.
5. Early return over deep nesting.
6. Explicit `match` for domain states.
7. `kanal` channels over mutexes for state sharing.

Avoid:

1. Oversized `mod.rs` files.
2. `String` as a universal error type.
3. `serde_json::Value` as a typed-model replacement.
4. `unwrap` anywhere in production code.
5. Overly broad mutable global state.
6. Wildcard imports in production code.

## New Feature Baseline 🚀

When adding a new subsystem or major feature:

1. Define its module boundary and responsibility first.
2. Put shared dependencies at workspace level.
3. Apply the full lint baseline.
4. Use `anyhow` + `tracing`.
5. Use `kanal` channels for cross-module communication; avoid new mutexes.
6. Use `pub(crate)` throughout; only promote to `pub` deliberately.
7. Include unit tests and, where applicable, integration tests.
8. No panic-based normal error handling.

## PR Review Checklist ✅

1. Is the module's responsibility clear and singular?
2. Is visibility minimal (`pub(crate)` preferred over `pub`)?
3. Are names domain-precise?
4. Are errors propagated with context, not swallowed or `expect`-ed?
5. Any unjustified `unwrap`/`expect`/`panic!` in runtime code?
6. Are logs level-appropriate and useful?
7. Is external data typed and validated at the boundary?
8. Is concurrency via channels, not mutexes?
9. Are there no wildcard imports in production code?
10. Are behaviour-focused tests included?

## Quick Module Template

```rust
mod types;

use anyhow::{Context, Result};
use tracing::info;

pub(crate) use types::MyEvent;

pub(crate) fn start() -> Result<kanal::Receiver<MyEvent>> {
    let (tx, rx) = kanal::unbounded();
    // ...
    info!("my_module started");
    Ok(rx)
}
```

Suggested layout for a subsystem:

```text
src/
  my_module/
    mod.rs      ← public(crate) API surface only
    types.rs    ← domain types
    worker.rs   ← implementation detail
```

## Final Note

This guide is intentionally stricter than parts of the current codebase.
For new work, prefer consistency over historical exceptions.
