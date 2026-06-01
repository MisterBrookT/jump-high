# Jump High 🐕

A **Jump King-style** terminal platformer built with Rust + ratatui.

Press SPACE to start charging, press SPACE again to jump. Miss a platform and you fall all the way down. No checkpoints, no mercy — just you, a pixel-art dog, and gravity.

## How to Play

The core mechanic is simple but punishing:

1. **Press SPACE** to start charging (power bar fills automatically)
2. **Press ←/→** while charging to aim your jump direction
3. **Press SPACE again** to jump — the longer you waited, the higher you go
4. Land on a platform or **fall all the way down** to whatever catches you

There is no game over. You just fall and try again. Like Jump King.

## Controls

| Key | Action |
|-----|--------|
| SPACE | Start charging (when grounded) |
| SPACE | Fire jump (when charging) |
| ← → | Aim direction (while charging) |
| p | Pause / Resume |
| q | Quit |

## Install

```bash
cargo install --path .
```

## Score

Score = maximum height reached. The platforms get trickier as you climb.

## License

MIT
