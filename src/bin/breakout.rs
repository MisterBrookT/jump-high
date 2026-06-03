use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::{
    io::stdout,
    time::{Duration, Instant},
};

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

const TICK_MS: u64 = 40;
const BRICK_ROWS: usize = 5;
const BRICK_COLS: usize = 8;
const BRICK_W: u16 = 6;
const BRICK_H: u16 = 2;
const PAD_W_NORMAL: u16 = 9;
const PAD_W_WIDE: u16 = 15;
const TRAIL_LEN: usize = 5;

fn brick_color(row: usize) -> Color {
    match row {
        0 => Color::Rgb(220, 60, 60),
        1 => Color::Rgb(220, 140, 60),
        2 => Color::Rgb(220, 220, 60),
        3 => Color::Rgb(60, 200, 60),
        _ => Color::Rgb(60, 140, 220),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PowerupKind {
    Life,
    Wide,
    Clear,
}

struct Powerup {
    kind: PowerupKind,
    x: f64,
    y: f64,
    tick_accum: u32,
}

struct Game {
    cols: u16,
    rows: u16,
    bricks: [[u8; BRICK_COLS]; BRICK_ROWS], // hp per brick
    ball_x: f64,
    ball_y: f64,
    ball_vx: f64,
    ball_vy: f64,
    trail: Vec<(f64, f64)>,
    pad_x: f64,
    pad_w: u16,
    lives: i32,
    score: u32,
    level: u32,
    paused: bool,
    game_over: bool,
    wide_ticks: u32,
    powerups: Vec<Powerup>,
    rng: u64,
    brick_origin_x: u16,
    brick_origin_y: u16,
}

impl Game {
    fn new(cols: u16, rows: u16) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        let mut g = Game {
            cols,
            rows,
            bricks: [[2u8; BRICK_COLS]; BRICK_ROWS],
            ball_x: cols as f64 / 2.0,
            ball_y: rows as f64 - 5.0,
            ball_vx: 0.7,
            ball_vy: -1.2,
            trail: Vec::new(),
            pad_x: cols as f64 / 2.0 - PAD_W_NORMAL as f64 / 2.0,
            pad_w: PAD_W_NORMAL,
            lives: 3,
            score: 0,
            level: 1,
            paused: false,
            game_over: false,
            wide_ticks: 0,
            powerups: Vec::new(),
            rng: seed,
            brick_origin_x: 0,
            brick_origin_y: 0,
        };
        g.compute_brick_origin();
        g
    }

    fn compute_brick_origin(&mut self) {
        let total_w = BRICK_COLS as u16 * BRICK_W;
        self.brick_origin_x = self.cols.saturating_sub(total_w) / 2;
        self.brick_origin_y = 2; // below HUD
    }

    fn lcg_next(&mut self) -> u64 {
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.rng >> 33
    }

    fn pad_center(&self) -> f64 {
        self.pad_x + self.pad_w as f64 / 2.0
    }

    fn reset_ball(&mut self) {
        self.ball_x = self.pad_center();
        self.ball_y = self.rows as f64 - 5.0;
        let speed = 1.2 + (self.level as f64 - 1.0) * 0.15;
        self.ball_vx = 0.7 * speed;
        self.ball_vy = -speed;
        self.trail.clear();
    }

    fn ball_speed(&self) -> f64 {
        (self.ball_vx * self.ball_vx + self.ball_vy * self.ball_vy).sqrt()
    }

    fn set_ball_speed(&mut self, spd: f64) {
        let cur = self.ball_speed();
        if cur > 0.001 {
            self.ball_vx = self.ball_vx / cur * spd;
            self.ball_vy = self.ball_vy / cur * spd;
        }
    }

    fn bricks_all_cleared(&self) -> bool {
        self.bricks.iter().all(|row| row.iter().all(|&hp| hp == 0))
    }

    fn next_level(&mut self) {
        self.level += 1;
        self.bricks = [[2u8; BRICK_COLS]; BRICK_ROWS];
        let speed = 1.2 + (self.level as f64 - 1.0) * 0.15;
        self.reset_ball();
        self.set_ball_speed(speed);
        self.powerups.clear();
        self.wide_ticks = 0;
        self.pad_w = PAD_W_NORMAL;
    }

    fn tick(&mut self) {
        if self.paused || self.game_over {
            return;
        }

        // Update wide powerup timer
        if self.wide_ticks > 0 {
            self.wide_ticks -= 1;
            if self.wide_ticks == 0 {
                self.pad_w = PAD_W_NORMAL;
            }
        }

        // Move powerups
        let mut to_remove_powerup = Vec::new();
        for (i, pw) in self.powerups.iter_mut().enumerate() {
            pw.tick_accum += 1;
            if pw.tick_accum >= 4 {
                pw.tick_accum = 0;
                pw.y += 1.0;
            }
            // Check if caught by paddle
            let pad_top = self.rows as f64 - 3.0;
            if pw.y >= pad_top && pw.y < pad_top + 1.0 {
                let px = pw.x as u16;
                if px >= self.pad_x as u16 && px < self.pad_x as u16 + self.pad_w {
                    // Apply powerup
                    match pw.kind {
                        PowerupKind::Life => {
                            self.lives += 1;
                        }
                        PowerupKind::Wide => {
                            self.pad_w = PAD_W_WIDE;
                            self.wide_ticks = 300;
                        }
                        PowerupKind::Clear => {
                            // Clear all HP=1 bricks
                            for row in &mut self.bricks {
                                for hp in row.iter_mut() {
                                    if *hp == 1 {
                                        *hp = 0;
                                    }
                                }
                            }
                        }
                    }
                    to_remove_powerup.push(i);
                }
            }
            // Off screen
            if pw.y >= self.rows as f64 {
                to_remove_powerup.push(i);
            }
        }
        // Remove in reverse order
        to_remove_powerup.sort_unstable();
        to_remove_powerup.dedup();
        for &i in to_remove_powerup.iter().rev() {
            self.powerups.remove(i);
        }

        // Store trail
        self.trail.push((self.ball_x, self.ball_y));
        if self.trail.len() > TRAIL_LEN {
            self.trail.remove(0);
        }

        // Move ball
        self.ball_x += self.ball_vx;
        self.ball_y += self.ball_vy;

        // Wall collisions (left/right)
        if self.ball_x < 0.0 {
            self.ball_x = 0.0;
            self.ball_vx = self.ball_vx.abs();
        }
        if self.ball_x >= self.cols as f64 - 1.0 {
            self.ball_x = self.cols as f64 - 2.0;
            self.ball_vx = -self.ball_vx.abs();
        }

        // Top wall
        if self.ball_y < 1.0 {
            self.ball_y = 1.0;
            self.ball_vy = self.ball_vy.abs();
        }

        // Paddle collision
        let pad_top = self.rows as f64 - 3.0;
        let pad_left = self.pad_x;
        let pad_right = self.pad_x + self.pad_w as f64;

        if self.ball_y >= pad_top
            && self.ball_y < pad_top + 1.0
            && self.ball_vy > 0.0
            && self.ball_x >= pad_left
            && self.ball_x < pad_right
        {
            self.ball_vy = -self.ball_vy.abs();
            let pad_third = self.pad_w as f64 / 3.0;
            let rel = self.ball_x - pad_left;
            let spd = self.ball_speed();
            if rel < pad_third {
                self.ball_vx = -0.8 * spd;
                self.ball_vy = -(1.0 - 0.64_f64).sqrt() * spd;
            } else if rel < pad_third * 2.0 {
                self.ball_vx *= 0.8;
                self.ball_vy = -(spd * spd - self.ball_vx * self.ball_vx).sqrt();
            } else {
                self.ball_vx = 0.8 * spd;
                self.ball_vy = -(1.0 - 0.64_f64).sqrt() * spd;
            }
            if self.ball_vy > 0.0 {
                self.ball_vy = -self.ball_vy;
            }
        }

        // Ball off bottom
        if self.ball_y >= self.rows as f64 - 1.0 {
            self.lives -= 1;
            if self.lives <= 0 {
                self.game_over = true;
            } else {
                self.reset_ball();
                self.powerups.clear();
            }
            return;
        }

        // Brick collision
        let bx = self.brick_origin_x;
        let by = self.brick_origin_y;

        let bx_f = bx as f64;
        let by_f = by as f64;
        let total_w = BRICK_COLS as f64 * BRICK_W as f64;
        let total_h = BRICK_ROWS as f64 * BRICK_H as f64;

        if self.ball_x >= bx_f
            && self.ball_x < bx_f + total_w
            && self.ball_y >= by_f
            && self.ball_y < by_f + total_h
        {
            let col = ((self.ball_x - bx_f) / BRICK_W as f64) as usize;
            let row = ((self.ball_y - by_f) / BRICK_H as f64) as usize;
            if row < BRICK_ROWS && col < BRICK_COLS && self.bricks[row][col] > 0 {
                self.bricks[row][col] -= 1;
                self.score += 10;
                self.ball_vy = -self.ball_vy;

                // Possibly drop powerup on level 2+
                if self.level >= 2 && self.bricks[row][col] == 0 {
                    let rnd = self.lcg_next() % 4;
                    if rnd == 0 {
                        let kind = match self.lcg_next() % 3 {
                            0 => PowerupKind::Life,
                            1 => PowerupKind::Wide,
                            _ => PowerupKind::Clear,
                        };
                        self.powerups.push(Powerup {
                            kind,
                            x: bx_f + col as f64 * BRICK_W as f64 + BRICK_W as f64 / 2.0,
                            y: by_f + row as f64 * BRICK_H as f64,
                            tick_accum: 0,
                        });
                    }
                }
            }
        }

        // Level up
        if self.bricks_all_cleared() {
            self.next_level();
        }
    }

    fn move_paddle(&mut self, dx: i64) {
        let new_x = self.pad_x + dx as f64 * 3.0;
        let max_x = self.cols as f64 - self.pad_w as f64;
        self.pad_x = new_x.max(0.0).min(max_x);
    }
}

fn draw(f: &mut Frame, game: &Game) {
    let area = f.area();

    // Background
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(10, 10, 20))),
        area,
    );

    // HUD
    let wide_str = if game.wide_ticks > 0 { " [WIDE]" } else { "" };
    let lives_str: String = "♥".repeat(game.lives.max(0) as usize);
    let hud_text = format!(
        " Level: {}  Score: {}  {}{}  [←→] Pad  [p] Pause  [r] Restart  [q] Quit",
        game.level, game.score, lives_str, wide_str
    );
    f.render_widget(
        Paragraph::new(hud_text)
            .style(Style::default().bg(Color::Rgb(20, 20, 40)).fg(Color::White)),
        Rect::new(area.x, area.y, area.width, 1),
    );

    if game.paused {
        f.render_widget(
            Paragraph::new("PAUSED — press p to resume")
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
    }

    if game.game_over {
        let lines = vec![
            Line::from(Span::styled(
                "GAME OVER",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("Score: {}", game.score),
                Style::default().fg(Color::White),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "r = restart   q = quit",
                Style::default().fg(Color::Rgb(160, 160, 180)),
            )),
        ];
        f.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2 - 2, area.width, 4),
        );
    }

    let buf = f.buffer_mut();

    // Draw bricks
    let bx = game.brick_origin_x;
    let by = game.brick_origin_y;
    for row in 0..BRICK_ROWS {
        for col in 0..BRICK_COLS {
            let hp = game.bricks[row][col];
            if hp == 0 {
                continue;
            }
            let tx = area.x + bx + col as u16 * BRICK_W;
            let ty = area.y + by + row as u16 * BRICK_H;
            let ch = if hp >= 2 { '▓' } else { '█' };
            let color = brick_color(row);
            for dy in 0..BRICK_H {
                for dx in 0..BRICK_W - 1 {
                    let px = tx + dx;
                    let py = ty + dy;
                    if px < area.x + area.width && py < area.y + area.height {
                        if let Some(cell) = buf.cell_mut((px, py)) {
                            cell.set_char(ch).set_fg(color);
                        }
                    }
                }
            }
        }
    }

    // Draw ball trail
    for (ti, &(tx, ty)) in game.trail.iter().enumerate() {
        let px = area.x + tx as u16;
        let py = area.y + ty as u16;
        if px < area.x + area.width && py > area.y && py < area.y + area.height {
            let alpha = (ti + 1) as u8 * 40;
            if let Some(cell) = buf.cell_mut((px, py)) {
                cell.set_char('·').set_fg(Color::Rgb(alpha, alpha, 200));
            }
        }
    }

    // Draw ball
    {
        let px = area.x + game.ball_x as u16;
        let py = area.y + game.ball_y as u16;
        if px < area.x + area.width && py < area.y + area.height {
            if let Some(cell) = buf.cell_mut((px, py)) {
                cell.set_char('●').set_fg(Color::White);
            }
        }
    }

    // Draw paddle
    {
        let py = area.y + game.rows - 3;
        for dx in 0..game.pad_w {
            let px = area.x + game.pad_x as u16 + dx;
            if px < area.x + area.width && py < area.y + area.height {
                if let Some(cell) = buf.cell_mut((px, py)) {
                    cell.set_char('█').set_fg(Color::Rgb(100, 180, 255));
                }
            }
        }
    }

    // Draw powerups
    for pw in &game.powerups {
        let px = area.x + pw.x as u16;
        let py = area.y + pw.y as u16;
        if px < area.x + area.width && py < area.y + area.height {
            let (ch, color) = match pw.kind {
                PowerupKind::Life => ('♥', Color::Red),
                PowerupKind::Wide => ('W', Color::Cyan),
                PowerupKind::Clear => ('C', Color::Yellow),
            };
            if let Some(cell) = buf.cell_mut((px, py)) {
                cell.set_char(ch).set_fg(color);
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _guard = Guard;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let mut game = Game::new(size.width, size.height);

    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &game))?;

        let elapsed = last_tick.elapsed();
        let wait = Duration::from_millis(TICK_MS).saturating_sub(elapsed);

        if event::poll(wait)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => {
                        let sz = terminal.size()?;
                        game = Game::new(sz.width, sz.height);
                        last_tick = Instant::now();
                    }
                    KeyCode::Char('p') => {
                        game.paused = !game.paused;
                    }
                    KeyCode::Left => game.move_paddle(-1),
                    KeyCode::Right => game.move_paddle(1),
                    _ => {}
                },
                Event::Resize(w, h) => {
                    game = Game::new(w, h);
                    last_tick = Instant::now();
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(TICK_MS) {
            game.tick();
            last_tick = Instant::now();
        }
    }

    Ok(())
}
