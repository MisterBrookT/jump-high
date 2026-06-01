# Jump High 🐕

A **Jump King-style** terminal platformer built with Rust + ratatui.

Hold SPACE to charge your jump, release to leap. Miss a platform and you fall all the way down. No checkpoints, no mercy — just you, a small dog, and gravity.

## How to Play

The core mechanic is simple but punishing:

1. **Hold SPACE** to charge your jump (a power bar fills up)
2. **Press ←/→** while charging to aim your jump direction
3. **Release SPACE** to jump — the longer you charged, the higher you go
4. Land on a platform or **fall all the way down** to whatever catches you

There is no game over. You just fall and try again. Like Jump King.

## Controls

| Key | Action |
|-----|--------|
| SPACE (hold) | Charge jump power |
| SPACE (release) | Jump |
| ← → | Aim direction (while charging) |
| q | Quit |

## Install

```bash
cargo install --path .
```

## Score

Score = maximum height reached. The platforms get trickier as you climb.

## License

MIT
