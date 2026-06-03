# 🐾 Paws Games

A community library of standalone terminal games that the [Paws](https://github.com/interesting-vibe-coding/paws) host can run in a tab while your AI agent works.

## Games

| ID | Name | Description |
|----|------|-------------|
| `jump-high` | 🐕 Dog Jump | Jump King-style platformer — charge, aim, and pray |
| `earth-online` | 🌍 Earth Online | Side quests for touching grass IRL |
| `tetris` | 🧱 Tetris | Classic block-stacking with levels and scoring |

## Install

Install any game as a standalone binary:

```bash
cargo install --git https://github.com/MisterBrookT/paws-games --bin jump-high
cargo install --git https://github.com/MisterBrookT/paws-games --bin earth-online
cargo install --git https://github.com/MisterBrookT/paws-games --bin tetris
```

Once on `PATH`, Paws discovers it via the registry and lets you launch it from the game picker.

## Contributing

Want to add a game? See [CONTRIBUTING.md](CONTRIBUTING.md) for the step-by-step guide and the game binary contract. Detailed technical docs live in [`docs/`](docs/).

## License

MIT
