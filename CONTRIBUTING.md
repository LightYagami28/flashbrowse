# Contributing to flashbrowse

Thanks for helping improve flashbrowse. This repository is a fork of
nhrbuilds/flashbrowse and keeps the Apache License 2.0.

## Development setup

The project is a Rust workspace. From the repository root:

```sh
cargo build --release
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

The CLI needs a Chromium browser. Use `flashbrowse setup` for the managed
browser, a browser already on `PATH`, or `CHROME_PATH` to point to one.
`flashbrowse doctor` reports the state of the local environment, and
`flashbrowse doctor --json` gives machine-readable output.

Keep changes in the crate that owns them:

- `flashbrowse-cdp` for transport, launcher and protocol types;
- `flashbrowse-core` for sessions, stealth, interception and compression;
- `flashbrowse-mcp` for the MCP stdio server;
- `flashbrowse-cli` for command-line behaviour.

## Pull requests

- One coherent change per pull request, with a clear description.
- State which commands you ran and their result.
- Build, tests, formatting and lint checks must pass without warnings or
  suppressions.
- Update `CHANGELOG.md` for user-visible changes.
- Do not open pull requests against upstream without prior approval.

## Commit style

Use conventional subjects in the form `type: description`, with `type` chosen
from `feat`, `fix`, `docs`, `refactor`, `test`, `perf` or `chore`.
