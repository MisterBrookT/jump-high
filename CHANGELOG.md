# Changelog

All notable changes to Paws Games are documented here.

## [v0.4.0] — 2026-06-03

### Added
- **Snake** — eat, grow, don't bite yourself; speed scales with score
- **2048** — slide tiles, merge numbers, reach 2048
- **Breakout** — smash bricks with a bouncing ball; power-ups, hard bricks, 3 lives
- **Space Invaders** — classic arcade shooter; blast the alien fleet before they land
- CI workflow: `cargo build`, `cargo clippy -D warnings`, `cargo fmt --check` on every push
- CI badge in README
- Homebrew formula updated to install all 7 games

### Changed
- Game library grows from 3 → 7 standalone terminal games

## [v0.3.0] — 2026-06-03

### Added
- **Dog Jump** — Jump King-style platformer; charge your jump, aim, pray
- **Earth Online** — side quests for touching grass IRL while your agent works
- **Tetris** — classic block-stacking with levels and scoring
- Game binary contract: each game renders in `(cols, rows-1)` leaving row 0 for the Paws HUD
- `CONTRIBUTING.md` and `docs/GAME_CONTRACT.md` for third-party game authors
- Homebrew formula (`paws-games`) for one-command install

[v0.4.0]: https://github.com/interesting-vibe-coding/paws-games/releases/tag/v0.4.0
[v0.3.0]: https://github.com/interesting-vibe-coding/paws-games/releases/tag/v0.3.0
