use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::{
    collections::VecDeque,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }
}

struct Game {
    snake: VecDeque<(i32, i32)>,
    dir: Dir,
    dir_queue: VecDeque<Dir>,
    food: (i32, i32),
    score: u32,
    dead: bool,
    rng: u64,
    field_w: i32,
    field_h: i32,
}

impl Game {
    fn new(field_w: i32, field_h: i32) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        let cx = field_w / 2;
        let cy = field_h / 2;
        let mut snake = VecDeque::new();
        snake.push_back((cx, cy));
        snake.push_back((cx - 1, cy));
        snake.push_back((cx - 2, cy));
        let mut g = Game {
            snake,
            dir: Dir::Right,
            dir_queue: VecDeque::new(),
            food: (0, 0),
            score: 0,
            dead: false,
            rng: seed,
            field_w,
            field_h,
        };
        g.place_food();
        g
    }

    fn lcg_next(&mut self) -> u64 {
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.rng >> 33
    }

    fn place_food(&mut self) {
        let mut attempts = 0u32;
        loop {
            let x = (self.lcg_next() % self.field_w as u64) as i32;
            let y = (self.lcg_next() % self.field_h as u64) as i32;
            if !self.snake.contains(&(x, y)) {
                self.food = (x, y);
                return;
            }
            attempts += 1;
            if attempts > 1000 {
                self.food = (0, 0);
                return;
            }
        }
    }

    fn tick(&mut self) {
        if self.dead {
            return;
        }

        // Consume queued direction
        while let Some(next) = self.dir_queue.front().copied() {
            if next == self.dir.opposite() {
                self.dir_queue.pop_front();
            } else {
                self.dir = self.dir_queue.pop_front().unwrap();
                break;
            }
        }

        let (hx, hy) = *self.snake.front().unwrap();
        let (nx, ny) = match self.dir {
            Dir::Up => (hx, hy - 1),
            Dir::Down => (hx, hy + 1),
            Dir::Left => (hx - 1, hy),
            Dir::Right => (hx + 1, hy),
        };

        // Wall collision
        if nx < 0 || nx >= self.field_w || ny < 0 || ny >= self.field_h {
            self.dead = true;
            return;
        }

        // Self collision (ignore tail tip since it moves)
        let body_len = self.snake.len();
        for (i, &seg) in self.snake.iter().enumerate() {
            if i == body_len - 1 {
                break;
            }
            if seg == (nx, ny) {
                self.dead = true;
                return;
            }
        }

        self.snake.push_front((nx, ny));

        if (nx, ny) == self.food {
            self.score += 10;
            self.place_food();
        } else {
            self.snake.pop_back();
        }
    }

    fn tick_ms(&self) -> u64 {
        let speed_ups = self.score / 50;
        150u64.saturating_sub(speed_ups as u64 * 10).max(60)
    }

    fn queue_dir(&mut self, d: Dir) {
        let last = self.dir_queue.back().copied().unwrap_or(self.dir);
        if d != last.opposite() && d != last && self.dir_queue.len() < 2 {
            self.dir_queue.push_back(d);
        }
    }
}

fn body_color(idx: usize, len: usize) -> Color {
    if idx == 0 {
        return Color::LightGreen;
    }
    let ratio = idx as f64 / len.max(1) as f64;
    // Fade from bright green (0,220,0) to dark green (0,60,0)
    let g = (220.0 - ratio * 160.0) as u8;
    Color::Rgb(0, g, 0)
}

fn draw(f: &mut Frame, game: &Game) {
    let area = f.area();
    let hud_h = 1u16;
    let field_h = area.height.saturating_sub(hud_h);
    let field_rect = Rect::new(area.x, area.y, area.width, field_h);
    let hud_rect = Rect::new(area.x, area.y + field_h, area.width, hud_h);

    // Background
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(10, 20, 10))),
        field_rect,
    );

    // HUD
    let tick_ms = game.tick_ms();
    let hud_text = format!(
        " Score: {:>5}  Tick: {}ms  [←↑↓→] Move  [r] Restart  [q] Quit",
        game.score, tick_ms
    );
    f.render_widget(
        Paragraph::new(hud_text)
            .style(Style::default().bg(Color::Rgb(20, 40, 20)).fg(Color::White)),
        hud_rect,
    );

    // Game-over overlay (widget before buffer_mut)
    if game.dead {
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
        let oy = area.y + field_h / 2 - 2;
        f.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            Rect::new(area.x, oy, area.width, 4),
        );
    }

    // Draw snake and food via buffer_mut
    let buf = f.buffer_mut();

    // Draw food
    let (fx, fy) = game.food;
    if fx >= 0 && fy >= 0 {
        let bx = area.x + fx as u16;
        let by = area.y + fy as u16;
        if bx < area.x + area.width && by < area.y + field_h {
            if let Some(cell) = buf.cell_mut((bx, by)) {
                cell.set_char('●').set_fg(Color::Red);
            }
        }
    }

    // Draw snake body (tail to head so head is drawn last)
    let len = game.snake.len();
    for (idx, &(sx, sy)) in game.snake.iter().enumerate() {
        let bx = area.x + sx as u16;
        let by = area.y + sy as u16;
        if sx < 0 || sy < 0 || bx >= area.x + area.width || by >= area.y + field_h {
            continue;
        }
        let ch = if idx == 0 { '◆' } else { '■' };
        let color = body_color(idx, len);
        if let Some(cell) = buf.cell_mut((bx, by)) {
            cell.set_char(ch).set_fg(color);
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
    let field_h = size.height.saturating_sub(1).max(5) as i32;
    let field_w = size.width.max(10) as i32;

    let mut game = Game::new(field_w, field_h);
    let mut last_tick = Instant::now();

    loop {
        // Check resize
        let sz = terminal.size()?;
        let new_fw = sz.width.max(10) as i32;
        let new_fh = sz.height.saturating_sub(1).max(5) as i32;
        if new_fw != game.field_w || new_fh != game.field_h {
            game = Game::new(new_fw, new_fh);
            last_tick = Instant::now();
        }

        terminal.draw(|f| draw(f, &game))?;

        let tick_ms = game.tick_ms();
        let elapsed = last_tick.elapsed();
        let wait = Duration::from_millis(tick_ms).saturating_sub(elapsed);

        if event::poll(wait)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => {
                        game = Game::new(game.field_w, game.field_h);
                        last_tick = Instant::now();
                    }
                    KeyCode::Up => game.queue_dir(Dir::Up),
                    KeyCode::Down => game.queue_dir(Dir::Down),
                    KeyCode::Left => game.queue_dir(Dir::Left),
                    KeyCode::Right => game.queue_dir(Dir::Right),
                    _ => {}
                },
                Event::Resize(w, h) => {
                    let new_fw = (w as i32).max(10);
                    let new_fh = ((h as i32) - 1).max(5);
                    game = Game::new(new_fw, new_fh);
                    last_tick = Instant::now();
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(tick_ms) {
            // Only tick if not dead (dead state keeps showing game-over overlay)
            if !game.dead {
                game.tick();
            }
            last_tick = Instant::now();
        }
    }

    Ok(())
}
