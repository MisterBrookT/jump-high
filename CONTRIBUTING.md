# Contributing a Game

## Steps

1. **Add your game binary** — create `src/bin/<id>.rs` following the [Game Contract](docs/GAME_CONTRACT.md).
2. **Build** — run `cargo build --release` and ensure it compiles with no errors.
3. **Open a PR** to this repo (`interesting-vibe-coding/paws-games`).
4. **Register in Paws** — add a matching entry to the `registry.toml` in the [paws repo](https://github.com/interesting-vibe-coding/paws) so the game appears in the picker.

## registry.toml entry format

Each game needs an entry like this in the paws repo's `registry.toml`:

```toml
[[game]]
id = "<id>"
name = "<Display Name>"
icon = "<emoji>"
cmd = "<id>"
install = "cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin <id>"
description = "One-line description of the game."
```

### Example

```toml
[[game]]
id = "tetris"
name = "Tetris"
icon = "🧱"
cmd = "tetris"
install = "cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin tetris"
description = "Classic block-stacking with levels and scoring."
```

## Conventions

- One file per game: `src/bin/<id>.rs` (self-contained, no lib dependencies beyond ratatui + crossterm).
- Use only the existing deps pinned in `Cargo.toml` — do not add new crates.
- The binary name = the filename stem = the `id` in the registry.
- Follow the terminal restore contract (see `docs/GAME_CONTRACT.md`).
