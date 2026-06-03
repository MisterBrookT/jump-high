# 🐾 Paws Games

A community library of standalone terminal games that the [Paws](https://github.com/interesting-vibe-coding/paws) host can run in a tab while your AI agent works.

## Games

| ID | Name | Description |
|----|------|-------------|
| `jump-high` | 🐕 Dog Jump | Jump King-style platformer — charge, aim, and pray |
| `earth-online` | 🌍 Earth Online | Side quests for touching grass IRL |
| `tetris` | 🧱 Tetris | Classic block-stacking with levels and scoring |
| `snake` | 🐍 Snake | Eat, grow, don’t bite yourself — speed scales with score |
| `2048` | 🎮 2048 | Slide tiles, merge numbers, reach 2048 |
| `breakout` | 🏓 Breakout | Smash bricks with a bouncing ball — power-ups, hard bricks, 3 lives |
| `space-invaders` | 👾 Space Invaders | Classic arcade shooter — blast the alien fleet before they land |

## Install

Install all games via Homebrew:

```bash
brew tap interesting-vibe-coding/paws
brew install paws-games
```

Or install individual games:

```bash
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin jump-high
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin earth-online
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin tetris
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin snake
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin 2048
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin breakout
cargo install --git https://github.com/interesting-vibe-coding/paws-games --bin space-invaders
```

Once on `PATH`, Paws discovers it via the registry and lets you launch it from the game picker.

## Contributing

Want to add a game? See [CONTRIBUTING.md](CONTRIBUTING.md) for the step-by-step guide and the game binary contract. Detailed technical docs live in [`docs/`](docs/).

## License

MIT
