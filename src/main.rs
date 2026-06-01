use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::{io::stdout, time::{Duration, Instant}};

const TICK_MS: u64 = 33;
const GRAVITY: f64 = 0.5;
const MAX_CHARGE: f64 = 30.0;
const CHARGE_RATE: f64 = 0.6;
const JUMP_POWER: f64 = -1.0; // multiplied by charge
const HORIZ_SPEED: f64 = 0.6; // multiplied by charge
const PLATFORM_WIDTH: u16 = 10;
const PLAYER_WIDTH: f64 = 6.0;

// Dog sprite (2 lines)
const DOG_LINE1: &str = "∪·ω·∪";
const DOG_LINE2: &str = "/|  |\\";

struct Platform {
    x: u16,
    y: f64,
    width: u16,
}

#[derive(PartialEq)]
enum State {
    Grounded,
    Charging,
    Airborne,
}

struct Game {
    px: f64,
    py: f64,
    vel_x: f64,
    vel_y: f64,
    charge: f64,
    dir: f64, // -1, 0, 1
    state: State,
    camera_y: f64,
    max_height: f64,
    platforms: Vec<Platform>,
    width: u16,
    height: u16,
}

impl Game {
    fn new(w: u16, h: u16) -> Self {
        let mut platforms = Vec::new();
        // Ground
        platforms.push(Platform { x: 0, y: (h as f64) - 2.0, width: w });
        // Generate platforms going up
        let mut rng_y = (h as f64) - 8.0;
        let mut side = false;
        while rng_y > -(h as f64 * 5.0) {
            let x = if side {
                (w as f64 * 0.55) as u16
            } else {
                (w as f64 * 0.1) as u16
            };
            platforms.push(Platform { x, y: rng_y, width: PLATFORM_WIDTH });
            rng_y -= 5.0 + (platforms.len() as f64 * 0.1).min(3.0);
            side = !side;
        }
        Self {
            px: w as f64 / 2.0 - 3.0,
            py: (h as f64) - 4.0,
            vel_x: 0.0,
            vel_y: 0.0,
            charge: 0.0,
            dir: 0.0,
            state: State::Grounded,
            camera_y: 0.0,
            max_height: 0.0,
            platforms,
            width: w,
            height: h,
        }
    }

    fn update(&mut self) {
        match self.state {
            State::Grounded => {}
            State::Charging => {
                self.charge = (self.charge + CHARGE_RATE).min(MAX_CHARGE);
            }
            State::Airborne => {
                self.vel_y += GRAVITY;
                self.px += self.vel_x;
                self.py += self.vel_y;

                // Wall collision
                if self.px < 0.0 {
                    self.px = 0.0;
                    self.vel_x = 0.0;
                }
                if self.px + PLAYER_WIDTH > self.width as f64 {
                    self.px = self.width as f64 - PLAYER_WIDTH;
                    self.vel_x = 0.0;
                }

                // Platform collision (only when falling)
                if self.vel_y > 0.0 {
                    for p in &self.platforms {
                        let plat_left = p.x as f64;
                        let plat_right = (p.x + p.width) as f64;
                        let player_left = self.px;
                        let player_right = self.px + PLAYER_WIDTH;

                        // Horizontal overlap
                        if player_right > plat_left && player_left < plat_right {
                            // Vertical: player bottom crosses platform top
                            let player_bottom = self.py + 2.0; // 2 lines tall
                            let prev_bottom = player_bottom - self.vel_y;
                            if prev_bottom <= p.y && player_bottom >= p.y {
                                self.py = p.y - 2.0;
                                self.vel_y = 0.0;
                                self.vel_x = 0.0;
                                self.state = State::Grounded;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Camera follows player
        let target_cam = self.py - self.height as f64 * 0.5;
        if target_cam < self.camera_y {
            self.camera_y += (target_cam - self.camera_y) * 0.1;
        }
        // Camera also follows down (when falling)
        if self.py - self.camera_y > self.height as f64 * 0.7 {
            self.camera_y = self.py - self.height as f64 * 0.7;
        }

        // Score = max height (inverted y)
        let height = -(self.py - (self.height as f64 - 4.0));
        if height > self.max_height {
            self.max_height = height;
        }

        // Generate more platforms above if needed
        let top_visible = self.camera_y - 10.0;
        let highest = self.platforms.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        if highest > top_visible {
            let mut y = highest - 6.0;
            let mut side = (self.platforms.len() % 2) == 0;
            for _ in 0..5 {
                let x = if side {
                    (self.width as f64 * 0.55) as u16
                } else {
                    (self.width as f64 * 0.1) as u16
                };
                self.platforms.push(Platform { x, y, width: PLATFORM_WIDTH });
                y -= 5.0 + (self.platforms.len() as f64 * 0.05).min(3.0);
                side = !side;
            }
        }
    }

    fn jump(&mut self) {
        if self.state == State::Charging && self.charge > 0.0 {
            self.vel_y = JUMP_POWER * self.charge * 0.4;
            self.vel_x = self.dir * HORIZ_SPEED * self.charge * 0.3;
            self.state = State::Airborne;
            self.charge = 0.0;
        }
    }

    fn start_charge(&mut self) {
        if self.state == State::Grounded {
            self.state = State::Charging;
            self.charge = 0.0;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let size = terminal.size()?;
    let mut game = Game::new(size.width, size.height);
    let mut space_held = false;

    loop {
        let frame_start = Instant::now();

        // Input
        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Key(key) => {
                    match (key.code, key.kind) {
                        (KeyCode::Char('q'), KeyEventKind::Press) => {
                            disable_raw_mode()?;
                            stdout().execute(LeaveAlternateScreen)?;
                            println!("Max height: {:.0}", game.max_height);
                            return Ok(());
                        }
                        (KeyCode::Char(' '), KeyEventKind::Press) => {
                            if !space_held {
                                space_held = true;
                                game.start_charge();
                            }
                        }
                        (KeyCode::Char(' '), KeyEventKind::Release) => {
                            space_held = false;
                            game.jump();
                        }
                        (KeyCode::Left, KeyEventKind::Press) => {
                            if game.state == State::Charging || game.state == State::Grounded {
                                game.dir = -1.0;
                            }
                        }
                        (KeyCode::Right, KeyEventKind::Press) => {
                            if game.state == State::Charging || game.state == State::Grounded {
                                game.dir = 1.0;
                            }
                        }
                        _ => {}
                    }
                }
                Event::Resize(w, h) => {
                    game.width = w;
                    game.height = h;
                }
                _ => {}
            }
        }

        game.update();

        // Render
        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            // Draw platforms
            for p in &game.platforms {
                let sy = (p.y - game.camera_y) as i16;
                if sy >= 0 && sy < area.height as i16 {
                    for dx in 0..p.width {
                        let px = p.x + dx;
                        if px < area.width {
                            if let Some(cell) = buf.cell_mut((px, sy as u16)) {
                                cell.set_char('█');
                                cell.set_fg(Color::Rgb(100, 180, 100));
                            }
                        }
                    }
                }
            }

            // Draw dog
            let dog_sx = game.px as u16;
            let dog_sy = (game.py - game.camera_y) as i16;

            // Line 1: face
            if dog_sy >= 0 && dog_sy < area.height as i16 {
                for (i, ch) in DOG_LINE1.chars().enumerate() {
                    let x = dog_sx + i as u16;
                    if x < area.width {
                        if let Some(cell) = buf.cell_mut((x, dog_sy as u16)) {
                            cell.set_char(ch);
                            cell.set_fg(Color::Rgb(255, 200, 100));
                        }
                    }
                }
            }
            // Line 2: legs
            let leg_y = dog_sy + 1;
            if leg_y >= 0 && leg_y < area.height as i16 {
                for (i, ch) in DOG_LINE2.chars().enumerate() {
                    let x = dog_sx + i as u16;
                    if x < area.width {
                        if let Some(cell) = buf.cell_mut((x, leg_y as u16)) {
                            cell.set_char(ch);
                            cell.set_fg(Color::Rgb(255, 200, 100));
                        }
                    }
                }
            }

            // Draw power bar when charging
            if game.state == State::Charging {
                let bar_y = dog_sy - 1;
                if bar_y >= 0 && bar_y < area.height as i16 {
                    let filled = ((game.charge / MAX_CHARGE) * 10.0) as usize;
                    let bar: String = format!("[{}{}]",
                        "■".repeat(filled),
                        "░".repeat(10 - filled),
                    );
                    for (i, ch) in bar.chars().enumerate() {
                        let x = dog_sx + i as u16;
                        if x < area.width {
                            if let Some(cell) = buf.cell_mut((x, bar_y as u16)) {
                                cell.set_char(ch);
                                cell.set_fg(if filled > 7 { Color::Red } else { Color::Yellow });
                            }
                        }
                    }
                }

                // Direction indicator
                let dir_str = match game.dir as i32 {
                    -1 => "← ",
                    1 => " →",
                    _ => "↑↑",
                };
                let dir_y = dog_sy - 2;
                if dir_y >= 0 && dir_y < area.height as i16 {
                    for (i, ch) in dir_str.chars().enumerate() {
                        let x = dog_sx + 2 + i as u16;
                        if x < area.width {
                            if let Some(cell) = buf.cell_mut((x, dir_y as u16)) {
                                cell.set_char(ch);
                                cell.set_fg(Color::Cyan);
                            }
                        }
                    }
                }
            }

            // HUD
            let hud = format!(" Height: {:.0}  Best: {:.0} ",
                -(game.py - (game.height as f64 - 4.0)).min(0.0),
                game.max_height
            );
            let hud_line = Line::from(hud).style(Style::default().fg(Color::White).bg(Color::DarkGray));
            Paragraph::new(hud_line).render(Rect::new(0, 0, area.width, 1), buf);

            // Controls hint
            let hint = " SPACE: charge/jump  ←→: aim  q: quit ";
            let hint_line = Line::from(hint).style(Style::default().fg(Color::Gray));
            Paragraph::new(hint_line)
                .render(Rect::new(0, area.height - 1, area.width, 1), buf);
        })?;

        // Frame timing
        let elapsed = frame_start.elapsed();
        if elapsed < Duration::from_millis(TICK_MS) {
            std::thread::sleep(Duration::from_millis(TICK_MS) - elapsed);
        }
    }
}
