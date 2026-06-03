# 🐾 Paws Games

A community library of standalone terminal games that the [Paws](https://github.com/interesting-vibe-coding/paws) host can run in a tab while your AI agent works.

[![CI](https://github.com/interesting-vibe-coding/paws-games/actions/workflows/ci.yml/badge.svg)](https://github.com/interesting-vibe-coding/paws-games/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE) [![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

## Games

| Binary | Name | Description |
|--------|------|-------------|
| `jump-high` | 🐕 Dog Jump | Jump King-style platformer — charge, aim, and pray |
| `earth-online` | 🌍 Earth Online | Side quests for touching grass IRL |
| `tetris` | 🧱 Tetris | Classic block-stacking with levels and scoring |
| `snake` | 🐍 Snake | Eat, grow, don't bite yourself — speed scales with score |
| `2048` | 🎮 2048 | Slide tiles, merge numbers, reach the 2048 tile |
| `breakout` | 🏓 Breakout | Smash bricks with a bouncing ball — power-ups, hard bricks, 3 lives |
| `space-invaders` | 👾 Space Invaders | Classic arcade shooter — blast the alien fleet before they land |

## Install

**Via Homebrew (recommended):**

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

Once on `PATH`, Paws auto-discovers games via the registry and lists them in the picker.

## Controls

| Game | Move | Action |
|------|------|--------|
| 🐕 Dog Jump | `← →` hold to charge | Release to jump |
| 🌍 Earth Online | `← →` | `Enter` / `Space` to complete quest |
| 🧱 Tetris | `← → ↓` | `↑` rotate, `Space` hard drop |
| 🐍 Snake | `← → ↑ ↓` | — |
| 🎮 2048 | `← → ↑ ↓` | `c` continue after win |
| 🏓 Breakout | `← →` | — |
| 👾 Space Invaders | `← →` | `Space` shoot |

All games: **`q`** or **`Esc`** quit · **`r`** restart

## Contributing

Want to add a game? See [CONTRIBUTING.md](CONTRIBUTING.md) for the step-by-step guide and the game binary contract. Detailed technical docs live in [`docs/`](docs/).

## License

MIT
