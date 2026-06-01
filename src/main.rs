use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use rand::Rng;
use ratatui::{prelude::*, widgets::*};
use std::{io::stdout, time::Duration};

const GRAVITY: f64 = 0.4;
const JUMP_VEL: f64 = -2.8;
const MOVE_SPEED: f64 = 1.5;
const PLATFORM_WIDTH: u16 = 7;
const TICK_MS: u64 = 33; // ~30fps

struct Platform {
    x: u16,
    y: f64, // world y
}

struct Game {
    player_x: f64,
    player_y: f64,
    vel_y: f64,
    vel_x: f64,
    camera_y: f64,
    score: u32,
    platforms: Vec<Platform>,
    width: u16,
    height: u16,
    game_over: bool,
}

impl Game {
    fn new(w: u16, h: u16) -> Self {
        let mut rng = rand::thread_rng();
        let mut platforms = Vec::new();
        // Generate initial platforms
        for i in 0..20 {
            platforms.push(Platform {
                x: rng.gen_range(0..w.saturating_sub(PLATFORM_WIDTH)),
                y: (h as f64) - (i as f64) * (h as f64 / 10.0),
            });
        }
        // Ground platform
        platforms.push(Platform { x: w / 2 - PLATFORM_WIDTH / 2, y: h as f64 - 2.0 });
        Self {
            player_x: w as f64 / 2.0,
            player_y: h as f64 - 4.0,
            vel_y: 0.0,
            vel_x: 0.0,
            camera_y: 0.0,
            score: 0,
            platforms,
            width: w,
            height: h,
            game_over: false,
        }
    }

    fn update(&mut self) {
        if self.game_over { return; }
        // Apply gravity
        self.vel_y += GRAVITY;
        self.player_y += self.vel_y;
        self.player_x += self.vel_x;
        self.vel_x *= 0.85; // friction

        // Wrap horizontally
        if self.player_x < 0.0 { self.player_x = self.width as f64 - 1.0; }
        if self.player_x >= self.width as f64 { self.player_x = 0.0; }

        // Platform collision (only when falling)
        if self.vel_y > 0.0 {
            for p in &self.platforms {
                let screen_py = p.y - self.camera_y;
                let screen_player = self.player_y - self.camera_y;
                if screen_player >= screen_py - 1.0
                    && screen_player <= screen_py + 0.5
                    && self.player_x >= p.x as f64 - 1.0
                    && self.player_x <= (p.x + PLATFORM_WIDTH) as f64
                {
                    self.vel_y = JUMP_VEL;
                    break;
                }
            }
        }

        // Scroll camera up
        let screen_y = self.player_y - self.camera_y;
        if screen_y < self.height as f64 * 0.4 {
            self.camera_y = self.player_y - self.height as f64 * 0.4;
        }

        // Update score
        let height = (-self.camera_y).max(0.0) as u32;
        if height > self.score { self.score = height; }

        // Generate new platforms above
        let top = self.camera_y;
        let highest = self.platforms.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        if highest > top - 5.0 {
            let mut rng = rand::thread_rng();
            let new_y = highest - rng.gen_range(3.0..6.0);
            self.platforms.push(Platform {
                x: rng.gen_range(0..self.width.saturating_sub(PLATFORM_WIDTH)),
                y: new_y,
            });
        }

        // Remove platforms far below
        let bottom = self.camera_y + self.height as f64 + 10.0;
        self.platforms.retain(|p| p.y < bottom);

        // Game over check
        if self.player_y - self.camera_y > self.height as f64 + 2.0 {
            self.game_over = true;
        }
    }

    fn resize(&mut self, w: u16, h: u16) {
        self.width = w;
        self.height = h;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let size = terminal.size()?;
    let mut game = Game::new(size.width, size.height);

    loop {
        // Input
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        disable_raw_mode()?;
                        stdout().execute(LeaveAlternateScreen)?;
                        println!("Final score: {}", game.score);
                        return Ok(());
                    }
                    KeyCode::Left => game.vel_x = -MOVE_SPEED,
                    KeyCode::Right => game.vel_x = MOVE_SPEED,
                    _ => {}
                }
            }
            if let Event::Resize(w, h) = event::read().unwrap_or(Event::FocusLost) {
                game.resize(w, h);
            }
        }

        // Update
        game.update();

        if game.game_over {
            disable_raw_mode()?;
            stdout().execute(LeaveAlternateScreen)?;
            println!("Game Over! Final score: {}", game.score);
            return Ok(());
        }

        // Render
        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            // Draw platforms
            for p in &game.platforms {
                let sy = (p.y - game.camera_y) as i16;
                if sy >= 0 && sy < area.height as i16 {
                    for dx in 0..PLATFORM_WIDTH {
                        let px = p.x + dx;
                        if px < area.width {
                            if let Some(cell) = buf.cell_mut((px, sy as u16)) {
                                cell.set_char('═');
                                cell.set_fg(Color::Green);
                            }
                        }
                    }
                }
            }

            // Draw player
            let px = game.player_x as u16;
            let py = (game.player_y - game.camera_y) as i16;
            if py >= 0 && py < area.height as i16 && px < area.width {
                if let Some(cell) = buf.cell_mut((px, py as u16)) {
                    cell.set_char('@');
                    cell.set_fg(Color::Yellow);
                }
            }

            // Draw score
            let score_text = format!(" Score: {} ", game.score);
            let score_span = Line::from(score_text).style(Style::default().fg(Color::White).bg(Color::DarkGray));
            let score_area = Rect::new(0, 0, area.width, 1);
            Paragraph::new(score_span).render(score_area, buf);
        })?;

        std::thread::sleep(Duration::from_millis(TICK_MS));
    }
}
