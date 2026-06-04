[![CI](https://github.com/interesting-vibe-coding/paws-games/actions/workflows/ci.yml/badge.svg)](https://github.com/interesting-vibe-coding/paws-games/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE) [![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

<div align="center">

# 🐾 Paws Games

**7 standalone terminal games for [Paws](https://github.com/interesting-vibe-coding/paws)**

Built with Rust + ratatui. Each game runs in a full terminal window while your AI agent thinks — a live HUD on the top row flashes when it needs you back.

</div>

## Games

| | Name | Description |
|--|------|-------------|
| 🐕 | **Dog Jump** | Jump King-style platformer — charge your jump, aim, and pray |
| 🌍 | **Earth Online** | Side quests for touching grass IRL while your agent works |
| 🧱 | **Tetris** | Classic block-stacking with levels and scoring |
| 🐍 | **Snake** | Eat, grow, don't bite yourself — speed scales with score |
| 🎮 | **2048** | Slide tiles, merge numbers, reach 2048 |
| 🏓 | **Breakout** | Smash bricks with a bouncing ball — power-ups, hard bricks, 3 lives |
| 👾 | **Space Invaders** | Classic arcade shooter — blast the alien fleet before they land |

## Install

**Via Homebrew (easiest):**

```bash
brew tap interesting-vibe-coding/paws
brew install paws-games
```

**Via Cargo (individual games):**

```bash
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin jump-high
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin earth-online
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin tetris
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin snake
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin 2048
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin breakout
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin space-invaders
```

Or just open **⤓ Install games** inside the Paws picker — it installs any game in-place with a live progress log.

## How it works

Each game is a standalone binary that reads terminal size and renders via ratatui + crossterm. Paws hosts the binary in a PTY and overlays a 1-row HUD showing your agent's status. Any binary that follows the [game contract](docs/GAME_CONTRACT.md) works — the registry in [paws](https://github.com/interesting-vibe-coding/paws/blob/main/registry.toml) is how Paws discovers games.

## Contributing

Want to add your own game? The bar is low:

1. Add `src/bin/<id>.rs` — one file, self-contained, uses only the pinned deps
2. Follow the [game contract](docs/GAME_CONTRACT.md) (restore terminal on exit)
3. Open a PR here, then add an entry to `registry.toml` in [paws](https://github.com/interesting-vibe-coding/paws)

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide.

## License

MIT
