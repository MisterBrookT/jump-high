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
const JUMP_POWER: f64 = -1.0;
const HORIZ_SPEED: f64 = 0.6;
const PLAYER_WIDTH: f64 = 7.0;
const PLAYER_HEIGHT: f64 = 4.0;

fn pseudo_rand(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state >> 33
}

fn rand_platform_width(state: &mut u64) -> u16 {
    (pseudo_rand(state) % 5 + 4) as u16 // 4..=8
}

fn rand_platform_x(state: &mut u64, area_width: u16, plat_width: u16) -> u16 {
    let max_x = area_width.saturating_sub(plat_width) as u64;
    if max_x == 0 { return 0; }
    (pseudo_rand(state) % max_x) as u16
}

// Pixel-art dog sprite colors
const C_BODY: Color = Color::Rgb(200, 150, 90);   // warm tan
const C_DARK: Color = Color::Rgb(120, 80, 40);    // dark outline/ears
const C_NOSE: Color = Color::Rgb(60, 40, 30);     // nose
const C_EYE: Color = Color::Rgb(255, 255, 255);   // eye highlight
const C_TAIL: Color = Color::Rgb(180, 130, 70);   // tail
const C_NONE: Color = Color::Reset;               // transparent (skip)

// Sprite: (char, fg_color) per cell. C_NONE = skip (transparent).
// Standing pose: 4 rows x 7 cols
//  Row 0:  . ▄ ▄ . . ▄ .    (ears + head top)
//  Row 1:  . █ ● █ ▄ █ .    (head: eye, nose, snout)
//  Row 2:  ▄ █ █ █ █ █ ╶    (body + tail stub)
//  Row 3:  . █ . █ . █ .    (legs)
const SPRITE_STAND: [[(char, Color); 7]; 4] = [
    [(' ', C_NONE), ('▄', C_DARK), ('▄', C_DARK), (' ', C_NONE), ('▄', C_DARK), ('▄', C_DARK), (' ', C_NONE)],
    [(' ', C_NONE), ('█', C_BODY), ('•', C_EYE),  ('█', C_BODY), ('•', C_EYE),  ('█', C_BODY), (' ', C_NONE)],
    [(' ', C_NONE), ('█', C_BODY), ('█', C_BODY), ('▀', C_NOSE), ('█', C_BODY), ('█', C_BODY), (' ', C_NONE)],
    [(' ', C_NONE), ('█', C_DARK), (' ', C_NONE), ('█', C_DARK), (' ', C_NONE), ('█', C_DARK), (' ', C_NONE)],
];

// Jumping pose: 4 rows x 7 cols (legs tucked, ears up)
//  Row 0:  . █ █ . . █ .    (ears tall)
//  Row 1:  . █ ● █ ▄ █ .    (head)
//  Row 2:  ▄ █ █ █ █ █ ─    (body + tail out)
//  Row 3:  . . ▀ ▀ ▀ . .    (tucked legs)
const SPRITE_JUMP: [[(char, Color); 7]; 4] = [
    [(' ', C_NONE), ('█', C_DARK), ('█', C_DARK), (' ', C_NONE), (' ', C_NONE), ('█', C_DARK), (' ', C_NONE)],
    [(' ', C_NONE), ('█', C_BODY), ('•', C_EYE),  ('█', C_BODY), ('▄', C_NOSE), ('█', C_BODY), (' ', C_NONE)],
    [('▄', C_BODY), ('█', C_BODY), ('█', C_BODY), ('█', C_BODY), ('█', C_BODY), ('█', C_BODY), ('─', C_TAIL)],
    [(' ', C_NONE), (' ', C_NONE), ('▀', C_BODY), ('▀', C_BODY), ('▀', C_BODY), (' ', C_NONE), (' ', C_NONE)],
];

// Standing facing RIGHT (nose points right, tail on left)
const SPRITE_STAND_RIGHT: [[(char, Color); 7]; 4] = [
    [(' ', C_NONE), (' ', C_NONE), (' ', C_NONE), ('▄', C_DARK), ('▄', C_DARK), ('▄', C_DARK), (' ', C_NONE)],
    [(' ', C_NONE), ('▄', C_BODY), ('█', C_BODY), ('█', C_BODY), ('•', C_EYE),  ('█', C_BODY), ('▶', C_NOSE)],
    [('╰', C_TAIL), ('█', C_BODY), ('█', C_BODY), ('█', C_BODY), ('█', C_BODY), ('█', C_BODY), (' ', C_NONE)],
    [(' ', C_NONE), ('█', C_DARK), (' ', C_NONE), ('█', C_DARK), (' ', C_NONE), ('█', C_DARK), (' ', C_NONE)],
];

// Standing facing LEFT (same as original SPRITE_STAND — nose points left, tail on right)
const SPRITE_STAND_LEFT: [[(char, Color); 7]; 4] = [
    [(' ', C_NONE), ('▄', C_DARK), ('▄', C_DARK), ('▄', C_DARK), (' ', C_NONE), (' ', C_NONE), (' ', C_NONE)],
    [('◀', C_NOSE), ('█', C_BODY), ('•', C_EYE),  ('█', C_BODY), ('█', C_BODY), ('▄', C_BODY), (' ', C_NONE)],
    [(' ', C_NONE), ('█', C_BODY), ('█', C_BODY), ('█', C_BODY), ('█', C_BODY), ('█', C_BODY), ('╯', C_TAIL)],
    [(' ', C_NONE), ('█', C_DARK), (' ', C_NONE), ('█', C_DARK), (' ', C_NONE), ('█', C_DARK), (' ', C_NONE)],
];

// Platform styles
const PLAT_STYLES: &[&[char]] = &[
    &['▓', '▓', '▒', '▓', '▓', '▒', '▓', '▓', '▒', '▓'],
    &['░', '▓', '█', '▓', '░', '▓', '█', '▓', '░', '▓'],
    &['▓', '█', '▓', '█', '▓', '█', '▓', '█', '▓', '█'],
];

const PLAT_COLORS: &[(u8, u8, u8)] = &[
    (220, 130, 50),
    (200, 110, 40),
    (180, 100, 30),
];

const GROUND_CHAR: char = '▓';
const GROUND_TOP: char = '▔';

struct Platform {
    x: u16,
    y: f64,
    width: u16,
    style: usize,
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
    dir: f64,
    state: State,
    camera_y: f64,
    max_height: f64,
    platforms: Vec<Platform>,
    width: u16,
    height: u16,
    paused: bool,
    space_count: u32,
    ticks_since_space: u32,
    rng_state: u64,
}

impl Game {
    fn new(w: u16, h: u16) -> Self {
        let mut rng_state: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let mut platforms = Vec::new();
        // Ground (full-width)
        platforms.push(Platform { x: 0, y: (h as f64) - 2.0, width: w, style: 0 });
        // Generate platforms going up
        let mut rng_y = (h as f64) - 8.0;
        let mut idx = 1usize;
        while rng_y > -(h as f64 * 5.0) {
            let pw = rand_platform_width(&mut rng_state);
            let x = rand_platform_x(&mut rng_state, w, pw);
            platforms.push(Platform { x, y: rng_y, width: pw, style: idx % PLAT_STYLES.len() });
            rng_y -= 5.0 + (platforms.len() as f64 * 0.1).min(3.0);
            idx += 1;
        }
        Self {
            px: w as f64 / 2.0 - 3.0,
            py: (h as f64) - 2.0 - PLAYER_HEIGHT,
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
            paused: false,
            space_count: 0,
            ticks_since_space: 0,
            rng_state,
        }
    }

    fn update(&mut self) {
        if self.paused { return; }
        match self.state {
            State::Grounded => {}
            State::Charging => {
                self.charge = (self.charge + CHARGE_RATE).min(MAX_CHARGE);
                self.ticks_since_space += 1;
                // Release = no space byte for a while. Once key-repeat has kicked
                // in (>=2 bytes) gaps are tiny, so 4 ticks (~130ms) is snappy.
                // Before repeat starts, wait longer (~330ms) to not misfire during
                // the OS key-repeat initial delay.
                let thresh = if self.space_count >= 2 { 4 } else { 18 };
                if self.ticks_since_space >= thresh {
                    self.jump();
                }
            }
            State::Airborne => {
                self.vel_y += GRAVITY;
                self.px += self.vel_x;
                self.py += self.vel_y;

                if self.px < 0.0 {
                    self.px = 0.0;
                    self.vel_x = 0.0;
                }
                if self.px + PLAYER_WIDTH > self.width as f64 {
                    self.px = self.width as f64 - PLAYER_WIDTH;
                    self.vel_x = 0.0;
                }

                if self.vel_y > 0.0 {
                    for p in &self.platforms {
                        let plat_left = p.x as f64;
                        let plat_right = (p.x + p.width) as f64;
                        let player_left = self.px;
                        let player_right = self.px + PLAYER_WIDTH;

                        if player_right > plat_left && player_left < plat_right {
                            let player_bottom = self.py + PLAYER_HEIGHT;
                            let prev_bottom = player_bottom - self.vel_y;
                            if prev_bottom <= p.y && player_bottom >= p.y {
                                self.py = p.y - PLAYER_HEIGHT;
                                self.vel_y = 0.0;
                                self.vel_x = 0.0;
                                self.dir = 0.0;
                                self.state = State::Grounded;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Camera
        let target_cam = self.py - self.height as f64 * 0.5;
        if target_cam < self.camera_y {
            self.camera_y += (target_cam - self.camera_y) * 0.1;
        }
        if self.py - self.camera_y > self.height as f64 * 0.7 {
            self.camera_y = self.py - self.height as f64 * 0.7;
        }

        // Score
        let height = -(self.py - (self.height as f64 - 2.0 - PLAYER_HEIGHT));
        if height > self.max_height {
            self.max_height = height;
        }

        // Generate more platforms
        let top_visible = self.camera_y - 10.0;
        let highest = self.platforms.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        if highest > top_visible {
            let mut y = highest - 6.0;
            for _ in 0..5 {
                let pw = rand_platform_width(&mut self.rng_state);
                let x = rand_platform_x(&mut self.rng_state, self.width, pw);
                let style = self.platforms.len() % PLAT_STYLES.len();
                self.platforms.push(Platform { x, y, width: pw, style });
                y -= 5.0 + (self.platforms.len() as f64 * 0.05).min(3.0);
            }
        }
    }

    fn jump(&mut self) {
        if self.state == State::Charging && self.charge > 0.0 {
            self.vel_y = JUMP_POWER * self.charge * 0.2;
            self.vel_x = self.dir * HORIZ_SPEED * self.charge * 0.18;
            self.state = State::Airborne;
            self.charge = 0.0;
        }
    }

    fn toggle_pause(&mut self) {
        if self.paused {
            self.paused = false;
        } else {
            // Entering pause: cancel charge if charging
            if self.state == State::Charging {
                self.state = State::Grounded;
                self.charge = 0.0;
            }
            self.paused = true;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let size = terminal.size()?;
    let mut game = Game::new(size.width, size.height);

    loop {
        let frame_start = Instant::now();

        // Input
        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press { continue; }
                    match key.code {
                        KeyCode::Char('q') => {
                            disable_raw_mode()?;
                            stdout().execute(LeaveAlternateScreen)?;
                            println!("Max height: {:.0}", game.max_height);
                            return Ok(());
                        }
                        KeyCode::Char('p') => {
                            game.toggle_pause();
                        }
                        KeyCode::Char(' ') => {
                            if game.paused { continue; }
                            match game.state {
                                State::Grounded => {
                                    game.state = State::Charging;
                                    game.charge = 0.0;
                                    game.space_count = 0;
                                    game.ticks_since_space = 0;
                                    // dir is NOT reset — keeps the direction set by arrow keys
                                }
                                State::Charging => {
                                    // Held: key-repeat byte → keep charging, reset release timer
                                    game.space_count += 1;
                                    game.ticks_since_space = 0;
                                }
                                State::Airborne => {}
                            }
                        }
                        KeyCode::Left => {
                            if !game.paused && (game.state == State::Charging || game.state == State::Grounded) {
                                game.dir = -1.0;
                            }
                        }
                        KeyCode::Right => {
                            if !game.paused && (game.state == State::Charging || game.state == State::Grounded) {
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
            for (pi, p) in game.platforms.iter().enumerate() {
                let sy = (p.y - game.camera_y) as i16;
                if sy < 0 || sy >= area.height as i16 { continue; }
                let is_ground = pi == 0;
                let (r, g, b) = if is_ground { (90, 70, 50) } else { PLAT_COLORS[p.style] };

                if is_ground {
                    for dx in 0..p.width.min(area.width) {
                        let px = p.x + dx;
                        if px < area.width {
                            if let Some(cell) = buf.cell_mut((px, sy as u16)) {
                                cell.set_char(GROUND_TOP);
                                cell.set_fg(Color::Rgb(140, 110, 70));
                            }
                        }
                    }
                    let gy = sy + 1;
                    if gy >= 0 && gy < area.height as i16 {
                        for dx in 0..p.width.min(area.width) {
                            let px = p.x + dx;
                            if px < area.width {
                                if let Some(cell) = buf.cell_mut((px, gy as u16)) {
                                    cell.set_char(GROUND_CHAR);
                                    cell.set_fg(Color::Rgb(r, g, b));
                                }
                            }
                        }
                    }
                } else {
                    let style_chars = PLAT_STYLES[p.style];
                    for dx in 0..p.width {
                        let px = p.x + dx;
                        if px < area.width {
                            if let Some(cell) = buf.cell_mut((px, sy as u16)) {
                                cell.set_char(style_chars[dx as usize % style_chars.len()]);
                                cell.set_fg(Color::Rgb(r, g, b));
                            }
                        }
                    }
                }
            }

            // Draw dog sprite
            let dog_sx = game.px as u16;
            let dog_sy = (game.py - game.camera_y) as i16;
            let sprite = if game.state == State::Airborne {
                &SPRITE_JUMP
            } else {
                match game.dir as i32 {
                    -1 => &SPRITE_STAND_LEFT,
                    1 => &SPRITE_STAND_RIGHT,
                    _ => &SPRITE_STAND,
                }
            };

            for (row, line) in sprite.iter().enumerate() {
                let sy = dog_sy + row as i16;
                if sy < 0 || sy >= area.height as i16 { continue; }
                for (col, &(ch, color)) in line.iter().enumerate() {
                    if color == C_NONE { continue; }
                    let x = dog_sx + col as u16;
                    if x < area.width {
                        if let Some(cell) = buf.cell_mut((x, sy as u16)) {
                            cell.set_char(ch);
                            cell.set_fg(color);
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
            let cur_height = (-(game.py - (game.height as f64 - 2.0 - PLAYER_HEIGHT))).max(0.0);
            let hud = format!(" Height: {:.0}  Best: {:.0} ", cur_height, game.max_height);
            let hud_line = Line::from(hud).style(Style::default().fg(Color::White).bg(Color::DarkGray));
            Paragraph::new(hud_line).render(Rect::new(0, 0, area.width, 1), buf);

            // Controls hint
            let hint = " SPACE: charge → SPACE: jump  ←→: aim  p: pause  q: quit ";
            let hint_line = Line::from(hint).style(Style::default().fg(Color::Gray));
            Paragraph::new(hint_line).render(Rect::new(0, area.height - 1, area.width, 1), buf);

            // Pause banner
            if game.paused {
                let msg = "  ⏸  PAUSED  ⏸  ";
                let mx = area.width.saturating_sub(msg.len() as u16) / 2;
                let my = area.height / 2;
                for (i, ch) in msg.chars().enumerate() {
                    let x = mx + i as u16;
                    if x < area.width {
                        if let Some(cell) = buf.cell_mut((x, my)) {
                            cell.set_char(ch);
                            cell.set_fg(Color::White);
                            cell.set_bg(Color::Rgb(80, 40, 40));
                        }
                    }
                }
            }
        })?;

        // Frame timing
        let elapsed = frame_start.elapsed();
        if elapsed < Duration::from_millis(TICK_MS) {
            std::thread::sleep(Duration::from_millis(TICK_MS) - elapsed);
        }
    }
}
