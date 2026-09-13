# AGENTS.md

`rscat` is a Rust RSVP speed-reader TUI. The repo root is a Cargo workspace
(`resolver = "3"`, sole member `rscat/`); the root README just lists crates.
User-facing usage, options and key bindings live in
[`rscat/README.md`](rscat/README.md) — keep it in sync when changing the CLI or
key handling.

Module-level invariants (timing model, tokenizer, pivot alignment, ghosting)
are documented next to the code, in the relevant source files.

## Commands

- Run cargo from the **repo root** (the workspace root has no `[package]`).
- `cargo test` — tests are `#[cfg(test)]` modules beside each source file.
- `cargo clippy --all-targets` must stay warning-free; `cargo fmt` before finishing.
- `nix build .#rscat` / `nix run .#rscat` / `nix develop`. There is **no
  `packages.default`**, so a bare `nix build` fails. The first Nix build compiles
  every crate from source (slow); the `result` symlink is gitignored.
- Adding a crate: add it to root `Cargo.toml` `members`, then add a
  `mkPackage "<dir>"` entry to `packages` and `apps` in `flake.nix`.

## Version control: jj, not git

- jj-colocated (`.jj/` + `.git/`). Never `git commit` / `git add`; jj
  auto-snapshots the working copy.
- Finish every task with `jj desc -m "..."`: each task starts as an empty
  revision, so you describe `@`. Inspect with `jj status` / `jj log`.
- `git` is inspection-only. `flake.nix` / `flake.lock` must be git-tracked for
  `nix build` to see them; jj keeps the index in sync (`git ls-files` to check).

## Real-terminal gotcha

- `run()` uses `Viewport::Inline`, which sends a cursor-position query
  (`ESC[6n`). Under a bare PTY with no terminal emulator it panics after a timeout.
- Drive the real UI under `tmux`, then read it with `tmux capture-pane -p` /
  `-pe` (`-e` keeps SGR codes; e.g. count `38;5;8` for DarkGray ghost cells).
- Unit tests need no terminal: rendering is tested with
  `ratatui::backend::TestBackend` on exact cell symbols/colours.

## Conventions

- Tests assert exact values, not ranges or merely "doesn't panic".
- Timing constants cite their source (papers/URLs) in doc comments; no
  undocumented magic numbers.
