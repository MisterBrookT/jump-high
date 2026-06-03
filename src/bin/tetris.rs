use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::{io::stdout, time::{Duration, Instant}};

const WELL_W: usize = 10;
const WELL_H: usize = 20;
const TICK_MS: u64 = 16;

type Shape = [[bool; 4]; 4];

#[derive(Clone, Copy)]
struct Piece {
    shapes: [Shape; 4],
    color: Color,
}

const I: Piece = Piece {
    shapes: [
        [[false,false,false,false],[true,true,true,true],[false,false,false,false],[false,false,false,false]],
        [[false,false,true,false],[false,false,true,false],[false,false,true,false],[false,false,true,false]],
        [[false,false,false,false],[false,false,false,false],[true,true,true,true],[false,false,false,false]],
        [[false,true,false,false],[false,true,false,false],[false,true,false,false],[false,true,false,false]],
    ],
    color: Color::Cyan,
};
const O: Piece = Piece {
    shapes: [
        [[false,false,false,false],[false,true,true,false],[false,true,true,false],[false,false,false,false]],
        [[false,false,false,false],[false,true,true,false],[false,true,true,false],[false,false,false,false]],
        [[false,false,false,false],[false,true,true,false],[false,true,true,false],[false,false,false,false]],
        [[false,false,false,false],[false,true,true,false],[false,true,true,false],[false,false,false,false]],
    ],
    color: Color::Yellow,
};
const T: Piece = Piece {
    shapes: [
        [[false,false,false,false],[false,true,false,false],[true,true,true,false],[false,false,false,false]],
        [[false,true,false,false],[false,true,true,false],[false,true,false,false],[false,false,false,false]],
        [[false,false,false,false],[true,true,true,false],[false,true,false,false],[false,false,false,false]],
        [[false,true,false,false],[true,true,false,false],[false,true,false,false],[false,false,false,false]],
    ],
    color: Color::Magenta,
};
const S: Piece = Piece {
    shapes: [
        [[false,false,false,false],[false,true,true,false],[true,true,false,false],[false,false,false,false]],
        [[false,true,false,false],[false,true,true,false],[false,false,true,false],[false,false,false,false]],
        [[false,false,false,false],[false,true,true,false],[true,true,false,false],[false,false,false,false]],
        [[false,true,false,false],[false,true,true,false],[false,false,true,false],[false,false,false,false]],
    ],
    color: Color::Green,
};
const Z: Piece = Piece {
    shapes: [
        [[false,false,false,false],[true,true,false,false],[false,true,true,false],[false,false,false,false]],
        [[false,false,true,false],[false,true,true,false],[false,true,false,false],[false,false,false,false]],
        [[false,false,false,false],[true,true,false,false],[false,true,true,false],[false,false,false,false]],
        [[false,false,true,false],[false,true,true,false],[false,true,false,false],[false,false,false,false]],
    ],
    color: Color::Red,
};
const J: Piece = Piece {
    shapes: [
        [[false,false,false,false],[true,false,false,false],[true,true,true,false],[false,false,false,false]],
        [[false,true,true,false],[false,true,false,false],[false,true,false,false],[false,false,false,false]],
        [[false,false,false,false],[true,true,true,false],[false,false,true,false],[false,false,false,false]],
        [[false,true,false,false],[false,true,false,false],[true,true,false,false],[false,false,false,false]],
    ],
    color: Color::Blue,
};
const L: Piece = Piece {
    shapes: [
        [[false,false,false,false],[false,false,true,false],[true,true,true,false],[false,false,false,false]],
        [[false,true,false,false],[false,true,false,false],[false,true,true,false],[false,false,false,false]],
        [[false,false,false,false],[true,true,true,false],[true,false,false,false],[false,false,false,false]],
        [[true,true,false,false],[false,true,false,false],[false,true,false,false],[false,false,false,false]],
    ],
    color: Color::Rgb(255, 165, 0),
};

const PIECES: [Piece; 7] = [I, O, T, S, Z, J, L];

struct Game {
    well: [[Option<Color>; WELL_W]; WELL_H],
    cur: usize,
    rot: usize,
    cx: i32,
    cy: i32,
    next: usize,
    score: u32,
    lines: u32,
    level: u32,
    game_over: bool,
    paused: bool,
    rng: u64,
    drop_accum: f64,
}

impl Game {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        let mut g = Game {
            well: [[None; WELL_W]; WELL_H],
            cur: 0, rot: 0, cx: 3, cy: 0,
            next: 0, score: 0, lines: 0, level: 1,
            game_over: false, paused: false,
            rng: seed, drop_accum: 0.0,
        };
        g.cur = g.rand_piece();
        g.next = g.rand_piece();
        g.spawn();
        g
    }

    fn rand_piece(&mut self) -> usize {
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.rng >> 33) % 7) as usize
    }

    fn shape(&self) -> &Shape {
        &PIECES[self.cur].shapes[self.rot]
    }

    fn color(&self) -> Color {
        PIECES[self.cur].color
    }

    fn fits(&self, piece: usize, rot: usize, cx: i32, cy: i32) -> bool {
        let s = &PIECES[piece].shapes[rot];
        for r in 0..4 {
            for c in 0..4 {
                if s[r][c] {
                    let x = cx + c as i32;
                    let y = cy + r as i32;
                    if x < 0 || x >= WELL_W as i32 || y >= WELL_H as i32 {
                        return false;
                    }
                    if y >= 0 && self.well[y as usize][x as usize].is_some() {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn spawn(&mut self) {
        self.cx = 3;
        self.cy = -1;
        self.rot = 0;
        // Adjust cy so piece starts visible
        for r in 0..4 {
            for c in 0..4 {
                if PIECES[self.cur].shapes[0][r][c] {
                    let y = self.cy + r as i32;
                    if y < 0 { self.cy = -(r as i32); }
                }
            }
            break; // only check first occupied row
        }
        self.cy = 0;
        if !self.fits(self.cur, self.rot, self.cx, self.cy) {
            self.game_over = true;
        }
    }

    fn lock(&mut self) {
        let s = self.shape().clone();
        let col = self.color();
        for r in 0..4 {
            for c in 0..4 {
                if s[r][c] {
                    let y = self.cy + r as i32;
                    let x = self.cx + c as i32;
                    if y >= 0 && y < WELL_H as i32 && x >= 0 && x < WELL_W as i32 {
                        self.well[y as usize][x as usize] = Some(col);
                    }
                }
            }
        }
        self.clear_lines();
        self.cur = self.next;
        self.next = self.rand_piece();
        self.spawn();
    }

    fn clear_lines(&mut self) {
        let mut cleared = 0u32;
        let mut y = WELL_H as i32 - 1;
        while y >= 0 {
            if self.well[y as usize].iter().all(|c| c.is_some()) {
                for row in (1..=y as usize).rev() {
                    self.well[row] = self.well[row - 1];
                }
                self.well[0] = [None; WELL_W];
                cleared += 1;
            } else {
                y -= 1;
            }
        }
        if cleared > 0 {
            self.lines += cleared;
            self.score += match cleared {
                1 => 100 * self.level,
                2 => 300 * self.level,
                3 => 500 * self.level,
                _ => 800 * self.level,
            };
            self.level = self.lines / 10 + 1;
        }
    }

    fn move_piece(&mut self, dx: i32, dy: i32) -> bool {
        if self.fits(self.cur, self.rot, self.cx + dx, self.cy + dy) {
            self.cx += dx;
            self.cy += dy;
            true
        } else {
            false
        }
    }

    fn rotate(&mut self, dir: i32) {
        let new_rot = ((self.rot as i32 + dir).rem_euclid(4)) as usize;
        if self.fits(self.cur, new_rot, self.cx, self.cy) {
            self.rot = new_rot;
        } else if self.fits(self.cur, new_rot, self.cx - 1, self.cy) {
            self.cx -= 1;
            self.rot = new_rot;
        } else if self.fits(self.cur, new_rot, self.cx + 1, self.cy) {
            self.cx += 1;
            self.rot = new_rot;
        }
    }

    fn hard_drop(&mut self) {
        while self.fits(self.cur, self.rot, self.cx, self.cy + 1) {
            self.cy += 1;
            self.score += 2;
        }
        self.lock();
    }

    fn drop_interval(&self) -> f64 {
        (1000.0 - (self.level as f64 - 1.0) * 80.0).max(50.0)
    }

    fn tick(&mut self, dt_ms: f64) {
        if self.game_over || self.paused { return; }
        self.drop_accum += dt_ms;
        let interval = self.drop_interval();
        while self.drop_accum >= interval {
            self.drop_accum -= interval;
            if !self.move_piece(0, 1) {
                self.lock();
                self.drop_accum = 0.0;
                break;
            }
        }
    }

    fn restart(&mut self) {
        *self = Game::new();
    }

    fn ghost_y(&self) -> i32 {
        let mut gy = self.cy;
        while self.fits(self.cur, self.rot, self.cx, gy + 1) {
            gy += 1;
        }
        gy
    }
}

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut game = Game::new();
    let mut last = Instant::now();

    loop {
        let now = Instant::now();
        let dt = now.duration_since(last).as_millis() as f64;
        last = now;
        game.tick(dt);

        terminal.draw(|f| draw(f, &game))?;

        if event::poll(Duration::from_millis(TICK_MS))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                if game.game_over {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('r') => game.restart(),
                        _ => {}
                    }
                } else if game.paused {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('p') => game.paused = false,
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('p') => game.paused = true,
                        KeyCode::Left => { game.move_piece(-1, 0); }
                        KeyCode::Right => { game.move_piece(1, 0); }
                        KeyCode::Down => { if game.move_piece(0, 1) { game.score += 1; } }
                        KeyCode::Char(' ') => game.hard_drop(),
                        KeyCode::Up | KeyCode::Char('x') => game.rotate(1),
                        KeyCode::Char('z') => game.rotate(-1),
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn draw(f: &mut Frame, game: &Game) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(Color::Rgb(15, 15, 25))), area);

    // Board dimensions: each cell = 2 chars wide, 1 char tall
    let board_w = (WELL_W as u16) * 2 + 2;
    let board_h = WELL_H as u16 + 2;
    let info_w = 14u16;
    let total_w = board_w + info_w + 1;
    let ox = area.x + area.width.saturating_sub(total_w) / 2;
    let oy = area.y + area.height.saturating_sub(board_h) / 2;

    // Draw well border
    let well_rect = Rect::new(ox, oy, board_w, board_h);
    let border = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(80, 80, 100)))
        .title(" TETRIS ")
        .title_style(Style::default().fg(Color::Rgb(200, 200, 255)).add_modifier(Modifier::BOLD));
    f.render_widget(border, well_rect);

    // Info panel (widgets that use f.render_widget — do these BEFORE buffer_mut)
    let ix = ox + board_w + 1;
    let iy = oy;

    if ix + 10 <= area.x + area.width {
        let next_label = Paragraph::new("NEXT")
            .style(Style::default().fg(Color::Rgb(180, 180, 200)));
        f.render_widget(next_label, Rect::new(ix, iy, 10, 1));

        let stats_y = iy + 6;
        let stats = vec![
            Line::from(Span::styled(format!("Score: {}", game.score), Style::default().fg(Color::White))),
            Line::from(Span::styled(format!("Level: {}", game.level), Style::default().fg(Color::Rgb(180, 220, 180)))),
            Line::from(Span::styled(format!("Lines: {}", game.lines), Style::default().fg(Color::Rgb(180, 200, 220)))),
        ];
        if stats_y + 3 <= area.y + area.height {
            f.render_widget(Paragraph::new(stats), Rect::new(ix, stats_y, 13, 3));
        }

        let ctrl_y = stats_y + 5;
        if ctrl_y + 7 <= area.y + area.height {
            let ctrl = vec![
                Line::from(Span::styled("Controls:", Style::default().fg(Color::Rgb(140, 140, 160)))),
                Line::from(Span::raw("←→  Move")),
                Line::from(Span::raw("↓   Soft drop")),
                Line::from(Span::raw("SPC Hard drop")),
                Line::from(Span::raw("↑/x CW  z CCW")),
                Line::from(Span::raw("p   Pause")),
                Line::from(Span::raw("q   Quit")),
            ];
            f.render_widget(Paragraph::new(ctrl).style(Style::default().fg(Color::Rgb(120, 120, 140))), Rect::new(ix, ctrl_y, 14, 7));
        }
    }

    // Pause overlay
    if game.paused {
        let msg = Paragraph::new("⏸ PAUSED (p to resume)")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(msg, Rect::new(area.x, area.y + area.height / 2, area.width, 1));
    }

    // Game over overlay
    if game.game_over {
        let lines = vec![
            Line::from(Span::styled("GAME OVER", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(format!("Score: {}", game.score), Style::default().fg(Color::White))),
            Line::raw(""),
            Line::from(Span::styled("r = restart  q = quit", Style::default().fg(Color::Rgb(160, 160, 180)))),
        ];
        f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), Rect::new(area.x, area.y + area.height / 2 - 2, area.width, 4));
    }

    // Now use buffer_mut for direct cell drawing (well, ghost, current piece, next piece preview)
    let buf = f.buffer_mut();

    // Draw well contents
    for r in 0..WELL_H {
        for c in 0..WELL_W {
            let x = ox + 1 + (c as u16) * 2;
            let y = oy + 1 + r as u16;
            if x + 1 >= area.x + area.width || y >= area.y + area.height { continue; }
            if let Some(color) = game.well[r][c] {
                buf.get_mut(x, y).set_char('█').set_fg(color);
                buf.get_mut(x + 1, y).set_char('█').set_fg(color);
            }
        }
    }

    // Draw ghost
    if !game.game_over && !game.paused {
        let gy = game.ghost_y();
        let s = game.shape();
        for r in 0..4 {
            for c in 0..4 {
                if s[r][c] {
                    let wr = gy + r as i32;
                    let wc = game.cx + c as i32;
                    if wr >= 0 && wr < WELL_H as i32 && wc >= 0 && wc < WELL_W as i32 {
                        let x = ox + 1 + (wc as u16) * 2;
                        let y = oy + 1 + wr as u16;
                        if x + 1 < area.x + area.width && y < area.y + area.height {
                            buf.get_mut(x, y).set_char('░').set_fg(Color::DarkGray);
                            buf.get_mut(x + 1, y).set_char('░').set_fg(Color::DarkGray);
                        }
                    }
                }
            }
        }
    }

    // Draw current piece
    if !game.game_over {
        let s = game.shape();
        let col = game.color();
        for r in 0..4 {
            for c in 0..4 {
                if s[r][c] {
                    let wr = game.cy + r as i32;
                    let wc = game.cx + c as i32;
                    if wr >= 0 && wr < WELL_H as i32 && wc >= 0 && wc < WELL_W as i32 {
                        let x = ox + 1 + (wc as u16) * 2;
                        let y = oy + 1 + wr as u16;
                        if x + 1 < area.x + area.width && y < area.y + area.height {
                            buf.get_mut(x, y).set_char('█').set_fg(col);
                            buf.get_mut(x + 1, y).set_char('█').set_fg(col);
                        }
                    }
                }
            }
        }
    }

    // Draw next piece preview (direct buffer)
    if ix + 10 <= area.x + area.width {
        let ns = &PIECES[game.next].shapes[0];
        let nc = PIECES[game.next].color;
        for r in 0..4 {
            for c in 0..4 {
                if ns[r][c] {
                    let x = ix + (c as u16) * 2;
                    let y = iy + 1 + r as u16;
                    if x + 1 < area.x + area.width && y < area.y + area.height {
                        buf.get_mut(x, y).set_char('█').set_fg(nc);
                        buf.get_mut(x + 1, y).set_char('█').set_fg(nc);
                    }
                }
            }
        }
    }
}
