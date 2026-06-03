use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::{io::stdout, time::Duration};

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

const CELL_W: u16 = 7;
const CELL_H: u16 = 3;

fn tile_color(val: u32) -> (Color, Color) {
    // Returns (bg, fg)
    match val {
        0 => (Color::Rgb(205, 193, 180), Color::Rgb(119, 110, 101)),
        2 => (Color::Rgb(238, 228, 218), Color::Rgb(119, 110, 101)),
        4 => (Color::Rgb(237, 224, 200), Color::Rgb(119, 110, 101)),
        8 => (Color::Rgb(242, 177, 121), Color::White),
        16 => (Color::Rgb(245, 149, 99), Color::White),
        32 => (Color::Rgb(246, 124, 95), Color::White),
        64 => (Color::Rgb(246, 94, 59), Color::White),
        128 => (Color::Rgb(237, 207, 114), Color::White),
        256 => (Color::Rgb(237, 204, 97), Color::White),
        512 => (Color::Rgb(237, 200, 80), Color::White),
        1024 => (Color::Rgb(237, 197, 63), Color::White),
        2048 => (Color::Rgb(237, 194, 46), Color::White),
        _ => (Color::Rgb(60, 58, 50), Color::White),
    }
}

fn slide_row(row: [u32; 4]) -> [u32; 4] {
    // Compress non-zeros to left
    let mut compressed = [0u32; 4];
    let mut ci = 0;
    for &v in row.iter() {
        if v != 0 {
            compressed[ci] = v;
            ci += 1;
        }
    }
    // Merge adjacent equal pairs (no double-merge)
    let mut merged = [0u32; 4];
    let mut mi = 0;
    let mut i = 0;
    while i < 4 {
        if compressed[i] == 0 {
            break;
        }
        if i + 1 < 4 && compressed[i] == compressed[i + 1] && compressed[i + 1] != 0 {
            merged[mi] = compressed[i] * 2;
            mi += 1;
            i += 2;
        } else {
            merged[mi] = compressed[i];
            mi += 1;
            i += 1;
        }
    }
    merged
}

struct Game {
    board: [[u32; 4]; 4],
    score: u32,
    best: u32,
    won: bool,
    keep_playing: bool,
    game_over: bool,
    rng: u64,
}

impl Game {
    fn new(best: u32) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        let mut g = Game {
            board: [[0; 4]; 4],
            score: 0,
            best,
            won: false,
            keep_playing: false,
            game_over: false,
            rng: seed,
        };
        g.add_tile();
        g.add_tile();
        g
    }

    fn lcg_next(&mut self) -> u64 {
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.rng >> 33
    }

    fn empty_cells(&self) -> Vec<(usize, usize)> {
        let mut v = Vec::new();
        for r in 0..4 {
            for c in 0..4 {
                if self.board[r][c] == 0 {
                    v.push((r, c));
                }
            }
        }
        v
    }

    fn add_tile(&mut self) {
        let empties = self.empty_cells();
        if empties.is_empty() {
            return;
        }
        let idx = (self.lcg_next() as usize) % empties.len();
        let (r, c) = empties[idx];
        // 10% chance of 4, 90% chance of 2
        self.board[r][c] = if self.lcg_next().is_multiple_of(10) {
            4
        } else {
            2
        };
    }

    fn move_left(&mut self) -> bool {
        let mut changed = false;
        for r in 0..4 {
            let orig = self.board[r];
            let new_row = slide_row(orig);
            if new_row != orig {
                changed = true;
                // Score: sum of merged tiles
                for c in 0..4 {
                    if new_row[c] != orig[c] && new_row[c] != 0 {
                        // Identify merges: new_row has higher values than orig
                    }
                }
                // Simple score: difference in sum (merges doubled tiles)
                let orig_sum: u32 = orig.iter().sum();
                let new_sum: u32 = new_row.iter().sum();
                self.score += new_sum.saturating_sub(orig_sum);
            }
            self.board[r] = new_row;
        }
        changed
    }

    fn move_right(&mut self) -> bool {
        // Reverse each row, slide, reverse back
        let mut changed = false;
        for r in 0..4 {
            let orig = self.board[r];
            let mut rev = orig;
            rev.reverse();
            let mut slid = slide_row(rev);
            slid.reverse();
            if slid != orig {
                changed = true;
                let orig_sum: u32 = orig.iter().sum();
                let new_sum: u32 = slid.iter().sum();
                self.score += new_sum.saturating_sub(orig_sum);
            }
            self.board[r] = slid;
        }
        changed
    }

    fn move_up(&mut self) -> bool {
        // Transpose, slide left, transpose back
        let mut changed = false;
        for c in 0..4 {
            let col = [
                self.board[0][c],
                self.board[1][c],
                self.board[2][c],
                self.board[3][c],
            ];
            let slid = slide_row(col);
            if slid != col {
                changed = true;
                let orig_sum: u32 = col.iter().sum();
                let new_sum: u32 = slid.iter().sum();
                self.score += new_sum.saturating_sub(orig_sum);
            }
            for (r, &val) in slid.iter().enumerate() {
                self.board[r][c] = val;
            }
        }
        changed
    }

    fn move_down(&mut self) -> bool {
        // Transpose, reverse, slide left, reverse, transpose back
        let mut changed = false;
        for c in 0..4 {
            let mut col = [
                self.board[0][c],
                self.board[1][c],
                self.board[2][c],
                self.board[3][c],
            ];
            col.reverse();
            let mut slid = slide_row(col);
            slid.reverse();
            let orig = [
                self.board[0][c],
                self.board[1][c],
                self.board[2][c],
                self.board[3][c],
            ];
            if slid != orig {
                changed = true;
                let orig_sum: u32 = orig.iter().sum();
                let new_sum: u32 = slid.iter().sum();
                self.score += new_sum.saturating_sub(orig_sum);
            }
            for (r, &val) in slid.iter().enumerate() {
                self.board[r][c] = val;
            }
        }
        changed
    }

    fn do_move(&mut self, dir: KeyCode) {
        if self.game_over {
            return;
        }
        if self.won && !self.keep_playing {
            return;
        }
        let changed = match dir {
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            _ => false,
        };
        if changed {
            if self.score > self.best {
                self.best = self.score;
            }
            self.add_tile();
            // Check win
            if !self.won {
                'outer: for r in 0..4 {
                    for c in 0..4 {
                        if self.board[r][c] == 2048 {
                            self.won = true;
                            break 'outer;
                        }
                    }
                }
            }
            // Check game over
            if self.empty_cells().is_empty() {
                self.game_over = !self.can_merge();
            }
        }
    }

    fn can_merge(&self) -> bool {
        for r in 0..4 {
            for c in 0..4 {
                if c + 1 < 4 && self.board[r][c] == self.board[r][c + 1] {
                    return true;
                }
                if r + 1 < 4 && self.board[r][c] == self.board[r + 1][c] {
                    return true;
                }
            }
        }
        false
    }
}

fn draw(f: &mut Frame, game: &Game) {
    let area = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(250, 248, 239))),
        area,
    );

    let grid_w = CELL_W * 4 + 1;
    let grid_h = CELL_H * 4 + 1;
    let hud_h = 3u16;
    let total_h = grid_h + hud_h;

    let ox = area.x + area.width.saturating_sub(grid_w) / 2;
    let oy = area.y + area.height.saturating_sub(total_h) / 2;

    // HUD
    let hud_rect = Rect::new(ox, oy, grid_w, hud_h);
    let hud_lines = vec![
        Line::from(vec![
            Span::styled(
                "2048",
                Style::default()
                    .fg(Color::Rgb(119, 110, 101))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Score: {:>6}", game.score),
                Style::default().fg(Color::Rgb(119, 110, 101)),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Best: {:>6}", game.best),
                Style::default().fg(Color::Rgb(119, 110, 101)),
            ),
        ]),
        Line::from(Span::styled(
            "[←↑↓→] Move  [r] Restart  [q] Quit",
            Style::default().fg(Color::Rgb(160, 150, 140)),
        )),
        Line::raw(""),
    ];
    f.render_widget(Paragraph::new(hud_lines), hud_rect);

    let board_oy = oy + hud_h;

    // Draw grid background
    let grid_rect = Rect::new(ox, board_oy, grid_w, grid_h);
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(187, 173, 160))),
        grid_rect,
    );

    // Draw each cell
    for r in 0..4usize {
        for c in 0..4usize {
            let val = game.board[r][c];
            let cx = ox + 1 + c as u16 * CELL_W;
            let cy = board_oy + 1 + r as u16 * CELL_H;
            let cell_rect = Rect::new(cx, cy, CELL_W - 1, CELL_H - 1);

            // Clamp to available area
            if cx + CELL_W - 1 > area.x + area.width || cy + CELL_H - 1 > area.y + area.height {
                continue;
            }

            let (bg, fg) = tile_color(val);
            let block = Block::default().style(Style::default().bg(bg));
            f.render_widget(block, cell_rect);

            if val > 0 {
                let text = val.to_string();
                let para = Paragraph::new(text)
                    .style(Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD))
                    .alignment(Alignment::Center);
                // Vertically center: CELL_H-1 = 2, use row 1 (0-indexed) for vertical center
                let text_rect = Rect::new(cx, cy + (CELL_H - 1) / 2, CELL_W - 1, 1);
                f.render_widget(para, text_rect);
            }
        }
    }

    // Win overlay
    if game.won && !game.keep_playing {
        let lines = vec![
            Line::from(Span::styled(
                "YOU WIN!",
                Style::default()
                    .fg(Color::Rgb(237, 194, 46))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "c = keep playing   r = restart   q = quit",
                Style::default().fg(Color::Rgb(119, 110, 101)),
            )),
        ];
        let overlay_y = area.y + area.height / 2 - 1;
        f.render_widget(
            Paragraph::new(lines)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Rgb(250, 248, 239))),
            Rect::new(area.x, overlay_y, area.width, 3),
        );
    }

    // Game over overlay
    if game.game_over {
        let lines = vec![
            Line::from(Span::styled(
                "GAME OVER",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("Score: {}", game.score),
                Style::default().fg(Color::Rgb(119, 110, 101)),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "r = restart   q = quit",
                Style::default().fg(Color::Rgb(160, 150, 140)),
            )),
        ];
        let overlay_y = area.y + area.height / 2 - 2;
        f.render_widget(
            Paragraph::new(lines)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Rgb(250, 248, 239))),
            Rect::new(area.x, overlay_y, area.width, 4),
        );
    }
}

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _guard = Guard;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut best = 0u32;
    let mut game = Game::new(best);

    loop {
        terminal.draw(|f| draw(f, &game))?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => {
                        best = game.best;
                        game = Game::new(best);
                    }
                    KeyCode::Char('c') => {
                        if game.won {
                            game.keep_playing = true;
                        }
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        game.do_move(key.code);
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(())
}
