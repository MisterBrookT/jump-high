# Game Binary Contract

## How Paws hosts a game

```
┌─────────────────────────────────────────┐
│  Paws terminal window                   │
├─────────────────────────────────────────┤ ← HUD (1 row, rendered by Paws)
│                                         │
│         Game PTY (child process)        │
│         rows = terminal_rows - 1        │
│                                         │
│                                         │
└─────────────────────────────────────────┘
```

Paws spawns the game binary as a **child process inside a PTY**. It overlays a one-row HUD on the **top row** of the terminal. The game sees `terminal_rows - 1` rows of usable space.

## Binary requirements

| Requirement | Detail |
|-------------|--------|
| Standalone | Single binary on `PATH`, no config files needed |
| Takes over the terminal | Enter raw mode + alternate screen on start |
| Reads stdin | All input comes through the PTY stdin |
| Quits on `q` | Must exit cleanly when the user presses `q` |
| Restores terminal | `disable_raw_mode()` + `LeaveAlternateScreen` on ANY exit path |
| Tolerates HUD row | Do **not** rely on the very top row — Paws owns it |
| 256-color safe | Stick to named colors or `Color::Rgb(...)` — no truecolor assumptions beyond 256 |
| Handle resize | Re-query terminal size on `Event::Resize` or use ratatui's auto-sized `Frame::area()` |

## Event loop pattern

```rust
fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    loop {
        terminal.draw(|f| { /* ... */ })?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Char('q') => break,
                    // ...
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
```

## Tips

- Use a **time accumulator** for gravity / game ticks — poll with a small timeout (~16ms) and accumulate elapsed time, don't tie logic to frame rate.
- Keep `Cargo.toml` deps as-is: `ratatui = "=0.29.0"`, `crossterm = "=0.28.1"`. No new crates.
- Test in a small terminal (e.g., 40×12) to make sure layout degrades gracefully.
- Use `Frame::area()` to get available size — never hardcode dimensions.
