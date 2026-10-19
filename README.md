# cognirail-rust-lint-config

Opinionated **clippy + rustfmt** baseline for Rust projects — the Rust sister of
[`@cognirail/eslint-config`](https://github.com/cognirail/eslint-config),
[`cognirail-go-lint-config`](https://github.com/cognirail/go-lint-config),
[`cognirail-python-lint-config`](https://github.com/cognirail/python-lint-config) and
[`cognirail-bash-lint-config`](https://github.com/cognirail/bash-lint-config).

Designed as **AI coding guardrails**: `clippy::all` + `clippy::pedantic` denied, panics and unfinished code denied,
a short named allow list, and **no blanket escape hatches**. One command, zero config, no dependencies.

## What it runs

| Step | Purpose |
| ---- | ------- |
| zero-override guard | Refuses to run if the project has `clippy.toml` / `rustfmt.toml` (dotted or not) or a `[lints]` / `[workspace.lints]` table in `Cargo.toml` |
| escape-hatch guard | Rejects `#[allow(..)]` anywhere and crate-level `#![allow(..)]` / `#![expect(..)]` |
| baseline sync | Writes `clippy.toml` and `rustfmt.toml` to `.cognirail/` (ships its own `.gitignore`) |
| `cargo fmt --check` | Formatting with the managed `rustfmt.toml` |
| `cargo clippy --all-targets --all-features` | Lint levels passed on the command line; thresholds read from the managed `clippy.toml` via `CLIPPY_CONF_DIR` |

Both cargo commands run as `cargo +1.99.1 …`, so every consumer lints with the same clippy and rustfmt.

## Requirements

- **rustup** with the pinned toolchain and its lint components:

```bash
rustup toolchain install 1.99.1 --profile minimal -c clippy -c rustfmt
```

## Quick start

```bash
cargo install --git https://github.com/cognirail/rust-lint-config --tag v0.1.0
cognirail-rust-lint-config check     # report only
cognirail-rust-lint-config fix       # cargo fmt + clippy --fix, then re-check
cognirail-rust-lint-config version   # print the pinned toolchain
```

## Exceptions

Rust has no path-based exclusion in clippy, so the exception mechanism is the language's own: an **item-level**
`#[expect(lint, reason = "...")]`.

- `clippy::allow_attributes` forbids `#[allow]`, so every exception is an `expect` — it fails once the code no longer
  needs it, so stale exceptions clean themselves up.
- `clippy::allow_attributes_without_reason` forces the `reason`.
- The guard rejects crate-level `#![expect(..)]`, which would switch a rule off for a whole crate.

```rust
#[expect(clippy::print_stdout, reason = "CLI output is the product. Remove when: never.")]
fn print_report(report: &Report) { /* ... */ }
```

## Rule philosophy

- **All on, name the exceptions.** `warnings`, `clippy::all` and `clippy::pedantic` are denied; every named allow in
  `src/baseline.rs` has a reason and a deletion condition.
- **No panics in production code.** `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `indexing_slicing`,
  `dbg_macro` and `print_stdout` / `print_stderr` are denied; `unsafe_code` is denied.
- **Complexity ≈ the sister packages.** clippy itself advises against `cognitive_complexity`, so the limits are
  `excessive_nesting` 4, `too_many_arguments` 6 and `too_many_lines` 80 (matching the Python baseline).
- **Tests are tests.** `#[test]` / `#[cfg(test)]` code may `unwrap`, `expect`, `panic`, print and index — but is otherwise
  fully linted. Helpers outside `#[test]` functions (for example in `tests/`) are not exempt: return `io::Result` instead.

### Named baseline allows (`src/baseline.rs`)

| Lint | Why | Remove when |
| ---- | --- | ----------- |
| `clippy::missing_errors_doc`, `clippy::missing_panics_doc` | Presence of docs is not a quality signal; written docs are still checked | never |
| `clippy::must_use_candidate` | `#[must_use]` on every pure function is noise; `unused_must_use` covers types that matter | never |

## Development

```bash
cargo test                       # unit tests
cargo test -- --ignored          # smoke: runs the CLI against temp consumer projects (clean passes, three failure modes fail)
cargo run -- check               # the package lints itself with its own baseline
```

## Versioning

- Patch: reason wording, docs.
- Minor: toolchain bumps that do not add failing lints to a clean consumer.
- Major: new lints denied or toolchain bumps that can fail previously clean consumers.

## License

[ISC](./LICENSE)
