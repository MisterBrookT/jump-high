use crossterm::{
    event::{
        self, Event, KeyCode, KeyEventKind, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    terminal::{
        disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::{io::stdout, time::{Duration, Instant}};

const TICK_MS: u64 = 33;
const GRAVITY: f64 = 0.5;
const MAX_CHARGE: f64 = 30.0;
const TAP_HOP: f64 = 5.0; // a quick tap = a small, visible hop
const CHARGE_RATE: f64 = 0.6;
const JUMP_POWER: f64 = -1.0;
const HORIZ_SPEED: f64 = 1.0;
const AIR_SPEED: f64 = 2.5; // mid-air steering speed
const PLAYER_WIDTH: f64 = 7.0;
const PLAYER_HEIGHT: f64 = 4.0;

fn pseudo_rand(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state >> 33
}

fn rand_platform_width(state: &mut u64, difficulty: u8) -> u16 {
    let r = pseudo_rand(state);
    match difficulty {
        1 => (r % 10 + 12) as u16, // easy: 12..=21 (wide)
        2 => (r % 16 + 5) as u16,  // medium: 5..=20 (wide & narrow mixed)
        _ => (r % 5 + 4) as u16,   // hard: 4..=8 (narrow)
    }
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
    cur_height: f64,
    platforms: Vec<Platform>,
    width: u16,
    height: u16,
    paused: bool,
    space_count: u32,
    ticks_since_space: u32,
    difficulty: u8,
    rng_state: u64,
}

impl Game {
    fn new(w: u16, h: u16, difficulty: u8) -> Self {
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
            let pw = rand_platform_width(&mut rng_state, difficulty);
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
            cur_height: 0.0,
            platforms,
            width: w,
            height: h,
            paused: false,
            space_count: 0,
            ticks_since_space: 0,
            rng_state,
            difficulty,
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
                let quick_tap = self.space_count < 2; // no key-repeat seen → just a tap
                let thresh = if quick_tap { 18 } else { 4 };
                if self.ticks_since_space >= thresh {
                    if quick_tap {
                        // A tap is a tiny hop, not a charged jump — don't let the
                        // wait-for-release window pump up the charge.
                        self.charge = self.charge.min(TAP_HOP);
                    }
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
                                // keep self.dir — the dog stays facing the last jump direction
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

        // Score = height of the platform you're standing on (not the airborne float)
        if self.state == State::Grounded {
            self.cur_height = -(self.py - (self.height as f64 - 2.0 - PLAYER_HEIGHT));
            if self.cur_height > self.max_height {
                self.max_height = self.cur_height;
            }
        }

        // Generate more platforms
        let top_visible = self.camera_y - 10.0;
        let highest = self.platforms.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        if highest > top_visible {
            let mut y = highest - 6.0;
            for _ in 0..5 {
                let pw = rand_platform_width(&mut self.rng_state, self.difficulty);
                let x = rand_platform_x(&mut self.rng_state, self.width, pw);
                let style = self.platforms.len() % PLAT_STYLES.len();
                self.platforms.push(Platform { x, y, width: pw, style });
                y -= 5.0 + (self.platforms.len() as f64 * 0.05).min(3.0);
            }
        }
    }

    fn steer(&mut self, dir: f64) {
        if self.paused {
            return;
        }
        match self.state {
            // On the ground / charging: set the aim for the next jump.
            State::Grounded | State::Charging => self.dir = dir,
            // In the air: steer horizontally (turn mid-jump; ↑ straightens).
            State::Airborne => {
                self.dir = dir;
                self.vel_x = dir * AIR_SPEED;
            }
        }
    }

    fn jump(&mut self) {
        if self.state == State::Charging {
            let c = self.charge.max(2.0); // quick tap → small but visible hop
            self.vel_y = JUMP_POWER * c * 0.2;
            self.vel_x = self.dir * HORIZ_SPEED * c * 0.18;
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
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let _ = stdout().execute(PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
        ));
    }
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let size = terminal.size()?;
    let mut game = Game::new(size.width, size.height, 1);

    loop {
        let frame_start = Instant::now();

        // Input
        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Key(key) => {
                    let press = key.kind == KeyEventKind::Press;
                    let repeat = key.kind == KeyEventKind::Repeat;
                    let release = key.kind == KeyEventKind::Release;
                    match key.code {
                        KeyCode::Char('q') if press => {
                            if kitty {
                                let _ = stdout().execute(PopKeyboardEnhancementFlags);
                            }
                            disable_raw_mode()?;
                            stdout().execute(LeaveAlternateScreen)?;
                            println!("Max height: {:.0}", game.max_height);
                            return Ok(());
                        }
                        KeyCode::Char('p') if press => game.toggle_pause(),
                        KeyCode::Char(c @ '1'..='3') if press => {
                            game = Game::new(game.width, game.height, c as u8 - b'0');
                        }
                        KeyCode::Char(' ') => {
                            if game.paused {
                                continue;
                            }
                            if release {
                                // Real key-release (kitty) → jump now: snappy, taps = small hops
                                if game.state == State::Charging {
                                    game.jump();
                                }
                            } else if press || repeat {
                                match game.state {
                                    State::Grounded => {
                                        game.state = State::Charging;
                                        game.charge = 0.0;
                                        game.space_count = 0;
                                        game.ticks_since_space = 0;
                                        // dir is kept from the last jump / arrow keys
                                    }
                                    State::Charging => {
                                        game.space_count += 1;
                                        game.ticks_since_space = 0;
                                    }
                                    State::Airborne => {}
                                }
                            }
                        }
                        KeyCode::Up if press || repeat => game.steer(0.0),
                        KeyCode::Left if press || repeat => game.steer(-1.0),
                        KeyCode::Right if press || repeat => game.steer(1.0),
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
            let hud = format!(" Height: {:.0}  Best: {:.0}  Lv {} ", game.cur_height.max(0.0), game.max_height, game.difficulty);
            let hud_line = Line::from(hud).style(
                Style::default()
                    .fg(Color::Rgb(255, 245, 210))
                    .bg(Color::Rgb(70, 55, 40))
                    .add_modifier(Modifier::BOLD),
            );
            Paragraph::new(hud_line).render(Rect::new(0, 0, area.width, 1), buf);

            // Controls hint
            let hint = " SPACE charge · ←↑→ aim · 1/2/3 level · p pause · q quit ";
            let hint_line = Line::from(hint).style(Style::default().fg(Color::Rgb(205, 200, 190)));
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
