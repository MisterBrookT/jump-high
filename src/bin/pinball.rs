use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

const TICK: Duration = Duration::from_millis(33);
const GRAVITY: f64 = 0.15;
const TABLE_W: f64 = 40.0;
const TABLE_H: f64 = 30.0;
const FLIPPER_DUR: Duration = Duration::from_millis(150);
const BALL_R: f64 = 0.5;
const BUMPER_R: f64 = 1.2;

struct Bumper { x: f64, y: f64, lit: u8 }
struct Game {
    bx: f64, by: f64, vx: f64, vy: f64,
    score: u32, balls: u8, launched: bool, paused: bool, over: bool,
    left_flip: Option<Instant>, right_flip: Option<Instant>,
    bumpers: Vec<Bumper>,
}

impl Game {
    fn new() -> Self {
        Self {
            bx: TABLE_W - 2.0, by: TABLE_H - 3.0, vx: 0.0, vy: 0.0,
            score: 0, balls: 3, launched: false, paused: false, over: false,
            left_flip: None, right_flip: None,
            bumpers: vec![
                Bumper { x: 10.0, y: 8.0, lit: 0 },
                Bumper { x: 20.0, y: 6.0, lit: 0 },
                Bumper { x: 30.0, y: 9.0, lit: 0 },
                Bumper { x: 15.0, y: 14.0, lit: 0 },
                Bumper { x: 25.0, y: 12.0, lit: 0 },
            ],
        }
    }

    fn launch(&mut self) {
        if !self.launched && !self.over {
            self.bx = TABLE_W - 2.0;
            self.by = TABLE_H - 4.0;
            self.vx = -1.5;
            self.vy = -4.5;
            self.launched = true;
        }
    }

    fn reset_ball(&mut self) {
        self.launched = false;
        self.bx = TABLE_W - 2.0;
        self.by = TABLE_H - 3.0;
        self.vx = 0.0;
        self.vy = 0.0;
    }

    fn tick(&mut self) {
        if self.paused || self.over || !self.launched { return; }
        let now = Instant::now();

        // gravity
        self.vy += GRAVITY;
        self.bx += self.vx;
        self.by += self.vy;

        // wall collisions
        if self.bx <= 1.0 { self.bx = 1.0; self.vx = self.vx.abs() * 0.9; }
        if self.bx >= TABLE_W - 1.0 { self.bx = TABLE_W - 1.0; self.vx = -self.vx.abs() * 0.9; }
        if self.by <= 1.0 { self.by = 1.0; self.vy = self.vy.abs() * 0.9; }

        // drain check (bottom gap between flippers)
        let drain_left = 12.0;
        let drain_right = 28.0;
        if self.by >= TABLE_H - 1.0 {
            if self.bx > drain_left && self.bx < drain_right {
                // drained
                self.balls -= 1;
                if self.balls == 0 { self.over = true; } else { self.reset_ball(); }
                return;
            } else {
                self.by = TABLE_H - 1.0;
                self.vy = -self.vy.abs() * 0.8;
            }
        }

        // flipper collision
        let left_active = self.left_flip.map_or(false, |t| now.duration_since(t) < FLIPPER_DUR);
        let right_active = self.right_flip.map_or(false, |t| now.duration_since(t) < FLIPPER_DUR);

        let flipper_y = TABLE_H - 3.0;
        // left flipper zone: x 4..14, y ~flipper_y
        if left_active && self.by >= flipper_y - 1.0 && self.by <= flipper_y + 1.0
            && self.bx >= 4.0 && self.bx <= 14.0 {
            self.vy = -4.0;
            self.vx += 1.0;
        }
        // right flipper zone: x 26..36, y ~flipper_y
        if right_active && self.by >= flipper_y - 1.0 && self.by <= flipper_y + 1.0
            && self.bx >= 26.0 && self.bx <= 36.0 {
            self.vy = -4.0;
            self.vx -= 1.0;
        }

        // bumper collisions
        for b in &mut self.bumpers {
            let dx = self.bx - b.x;
            let dy = self.by - b.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < BALL_R + BUMPER_R {
                let nx = dx / dist;
                let ny = dy / dist;
                self.vx = nx * 3.0;
                self.vy = ny * 3.0;
                self.bx = b.x + nx * (BUMPER_R + BALL_R + 0.1);
                self.by = b.y + ny * (BUMPER_R + BALL_R + 0.1);
                b.lit = 8;
                self.score += 100;
            }
        }

        // decay lit
        for b in &mut self.bumpers { if b.lit > 0 { b.lit -= 1; } }

        // speed cap
        self.vx = self.vx.clamp(-6.0, 6.0);
        self.vy = self.vy.clamp(-6.0, 6.0);
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut game = Game::new();
    let mut last_tick = Instant::now();

    loop {
        if event::poll(Duration::from_millis(5))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('r') => { game = Game::new(); }
                    KeyCode::Char('p') => { game.paused = !game.paused; }
                    KeyCode::Char(' ') => { game.launch(); }
                    KeyCode::Left | KeyCode::Char('a') => {
                        game.left_flip = Some(Instant::now());
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        game.right_flip = Some(Instant::now());
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= TICK {
            game.tick();
            last_tick = Instant::now();
        }

        term.draw(|f| {
            let area = f.area();
            // We need at least 34 rows and 44 cols for the table
            let tw = (TABLE_W as u16) + 4;
            let th = (TABLE_H as u16) + 4;
            let ox = area.width.saturating_sub(tw) / 2;
            let oy = area.height.saturating_sub(th) / 2;
            let table_rect = Rect::new(ox, oy, tw.min(area.width), th.min(area.height));

            let mut lines: Vec<Line> = Vec::new();

            // HUD
            let hud = format!("  PINBALL   SCORE: {}   BALLS: {}  {}",
                game.score, game.balls,
                if game.paused { "[PAUSED]" } else if game.over { "[GAME OVER - R:restart Q:quit]" } else { "" });
            lines.push(Line::from(Span::styled(hud, Style::default().fg(Color::Rgb(245, 160, 50)))));
            lines.push(Line::from(""));

            let now = Instant::now();
            let left_up = game.left_flip.map_or(false, |t| now.duration_since(t) < FLIPPER_DUR);
            let right_up = game.right_flip.map_or(false, |t| now.duration_since(t) < FLIPPER_DUR);

            // Render table rows
            for row in 0..(TABLE_H as i32) {
                let mut chars: Vec<Span> = Vec::new();
                chars.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
                for col in 0..(TABLE_W as i32) {
                    let fx = col as f64;
                    let fy = row as f64;

                    // Check if ball here
                    let is_ball = (fx - game.bx).abs() < 1.0 && (fy - game.by).abs() < 0.8;
                    // Check bumper
                    let mut is_bumper = false;
                    let mut bumper_lit = false;
                    for b in &game.bumpers {
                        if (fx - b.x).abs() < 1.5 && (fy - b.y).abs() < 1.0 {
                            is_bumper = true;
                            bumper_lit = b.lit > 0;
                            break;
                        }
                    }
                    // Flipper display
                    let flipper_y = (TABLE_H - 3.0) as i32;
                    let is_left_flip = fy as i32 == flipper_y && col >= 4 && col <= 13;
                    let is_right_flip = fy as i32 == flipper_y && col >= 27 && col <= 36;
                    // Drain gap indicator
                    let is_drain = row == (TABLE_H as i32 - 1) && col > 12 && col < 28;

                    if is_ball && game.launched {
                        chars.push(Span::styled("●", Style::default().fg(Color::Cyan)));
                    } else if is_bumper {
                        let c = if bumper_lit { Color::Rgb(255, 200, 50) } else { Color::Rgb(245, 160, 50) };
                        chars.push(Span::styled("◉", Style::default().fg(c)));
                    } else if is_left_flip {
                        let ch = if left_up { "▔" } else { "_" };
                        chars.push(Span::styled(ch, Style::default().fg(Color::White)));
                    } else if is_right_flip {
                        let ch = if right_up { "▔" } else { "_" };
                        chars.push(Span::styled(ch, Style::default().fg(Color::White)));
                    } else if is_drain {
                        chars.push(Span::styled(" ", Style::default()));
                    } else if row == (TABLE_H as i32 - 1) {
                        chars.push(Span::styled("▄", Style::default().fg(Color::DarkGray)));
                    } else if col == 0 || col == (TABLE_W as i32 - 1) {
                        chars.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
                    } else if row == 0 {
                        chars.push(Span::styled("▀", Style::default().fg(Color::DarkGray)));
                    } else {
                        chars.push(Span::styled(" ", Style::default()));
                    }
                }
                chars.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
                lines.push(Line::from(chars));
            }

            // Bottom border
            let bot = "└".to_string() + &"─".repeat(TABLE_W as usize) + "┘";
            lines.push(Line::from(Span::styled(bot, Style::default().fg(Color::DarkGray))));

            // Controls help
            lines.push(Line::from(Span::styled(
                "  ←/a: left flip  →/l: right flip  SPACE: launch  p: pause  q: quit",
                Style::default().fg(Color::DarkGray),
            )));

            let para = Paragraph::new(lines)
                .block(Block::default().borders(Borders::NONE));
            f.render_widget(para, table_rect);
        })?;
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
