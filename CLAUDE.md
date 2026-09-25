# sdat2img-rust

Rust port of [xpirt/sdat2img](https://github.com/xpirt/sdat2img) (`sdat2img.py` 1.2).

## Language

- All project content is in **English**: README, docs, code comments, error messages, commit messages and PR descriptions.
- The maintainer may chat in Indonesian. Reply in their language in chat, but anything written into the repo stays in English.

## Layout

- `src/lib.rs`: transfer list parsing, rangesets, and image writing. Unit tests live here.
- `src/main.rs`: a thin CLI with the same arguments and messages as the Python original.

## Rules

- No external dependencies (std only) unless there's a strong reason.
- Output must stay byte-identical to `sdat2img.py` for valid input. Check behavior changes against the original.
- Before committing, run `cargo fmt`, `cargo clippy --all-targets` (it must report zero warnings) and `cargo test`.
