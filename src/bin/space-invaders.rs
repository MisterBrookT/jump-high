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

const TICK_MS: u64 = 33;
const ALIEN_COLS: usize = 5;
const ALIEN_ROWS: usize = 3;
const ALIEN_STEP_X: i32 = 6; // horizontal spacing between alien centres
const ALIEN_STEP_Y: i32 = 3; // vertical spacing between alien rows
const SHIELD_COUNT: usize = 3;
const SHIELD_W: usize = 6;
const SHIELD_H: usize = 2;

// Points per alien row (top → bottom)
const ALIEN_PTS: [u32; ALIEN_ROWS] = [30, 20, 10];

#[derive(Clone, Copy)]
struct Explosion {
    x: i32,
    y: i32,
    ticks: u8,
}

struct Game {
    cols: u16,
    rows: u16,
    // Fleet
    alive: [[bool; ALIEN_COLS]; ALIEN_ROWS],
    fleet_x: i32,
    fleet_y: i32,
    fleet_dx: i32,
    move_accum: u32,
    move_interval: u32,
    explosions: Vec<Explosion>,
    // Bullets
    player_bullet: Option<(i32, i32)>,
    alien_bullets: Vec<(i32, i32)>,
    alien_fire_accum: u32,
    // Player
    player_x: i32,
    // Shields [shield][row][col] = hp (0..=4)
    shields: [[[u8; SHIELD_W]; SHIELD_H]; SHIELD_COUNT],
    // State
    score: u32,
    lives: u32,
    level: u32,
    paused: bool,
    game_over: bool,
    cleared: bool,
    rng: u64,
}

impl Game {
    fn new(cols: u16, rows: u16) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        let fleet_w = (ALIEN_COLS as i32 - 1) * ALIEN_STEP_X;
        let fleet_x = (cols as i32 - fleet_w) / 2;
        Game {
            cols,
            rows,
            alive: [[true; ALIEN_COLS]; ALIEN_ROWS],
            fleet_x,
            fleet_y: 3,
            fleet_dx: 1,
            move_accum: 0,
            move_interval: 10,
            explosions: Vec::new(),
            player_bullet: None,
            alien_bullets: Vec::new(),
            alien_fire_accum: 0,
            player_x: cols as i32 / 2,
            shields: [[[4u8; SHIELD_W]; SHIELD_H]; SHIELD_COUNT],
            score: 0,
            lives: 3,
            level: 1,
            paused: false,
            game_over: false,
            cleared: false,
            rng: seed,
        }
    }

    fn lcg_next(&mut self) -> u64 {
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.rng >> 33
    }

    fn alive_count(&self) -> usize {
        self.alive
            .iter()
            .flat_map(|r| r.iter())
            .filter(|&&a| a)
            .count()
    }

    fn alien_pos(&self, row: usize, col: usize) -> (i32, i32) {
        let x = self.fleet_x + col as i32 * ALIEN_STEP_X;
        let y = self.fleet_y + row as i32 * ALIEN_STEP_Y;
        (x, y)
    }

    fn player_y(&self) -> i32 {
        self.rows as i32 - 3
    }

    fn shield_x(&self, s: usize) -> i32 {
        let total = SHIELD_COUNT as i32 * SHIELD_W as i32 + (SHIELD_COUNT as i32 - 1) * 4;
        let start = (self.cols as i32 - total) / 2;
        start + s as i32 * (SHIELD_W as i32 + 4)
    }

    fn shield_y(&self) -> i32 {
        self.rows as i32 - 7
    }

    fn next_level(&mut self) {
        self.level += 1;
        self.alive = [[true; ALIEN_COLS]; ALIEN_ROWS];
        let fleet_w = (ALIEN_COLS as i32 - 1) * ALIEN_STEP_X;
        self.fleet_x = (self.cols as i32 - fleet_w) / 2;
        self.fleet_y = 3;
        self.fleet_dx = 1;
        self.move_interval = 10u32.saturating_sub(self.level - 1).max(2);
        self.move_accum = 0;
        self.player_bullet = None;
        self.alien_bullets.clear();
        self.explosions.clear();
        self.cleared = false;
    }

    fn damage_shield(&mut self, bx: i32, by: i32) -> bool {
        let sy = self.shield_y();
        for s in 0..SHIELD_COUNT {
            let sx = self.shield_x(s);
            let rx = bx - sx;
            let ry = by - sy;
            if rx >= 0 && ry >= 0 && (rx as usize) < SHIELD_W && (ry as usize) < SHIELD_H {
                let hp = &mut self.shields[s][ry as usize][rx as usize];
                if *hp > 0 {
                    *hp -= 1;
                    return true;
                }
            }
        }
        false
    }

    fn tick(&mut self) {
        if self.paused || self.game_over || self.cleared {
            return;
        }

        // Age explosions
        for e in &mut self.explosions {
            e.ticks = e.ticks.saturating_sub(1);
        }
        self.explosions.retain(|e| e.ticks > 0);

        // Move fleet
        self.move_accum += 1;
        if self.move_accum >= self.move_interval {
            self.move_accum = 0;
            // Find actual left/right extents of alive aliens
            let mut min_col = ALIEN_COLS;
            let mut max_col = 0usize;
            for r in 0..ALIEN_ROWS {
                for c in 0..ALIEN_COLS {
                    if self.alive[r][c] {
                        if c < min_col {
                            min_col = c;
                        }
                        if c > max_col {
                            max_col = c;
                        }
                    }
                }
            }
            if min_col <= max_col {
                let left = self.fleet_x + min_col as i32 * ALIEN_STEP_X;
                let right = self.fleet_x + max_col as i32 * ALIEN_STEP_X;
                let hit_right = self.fleet_dx > 0 && right >= self.cols as i32 - 2;
                let hit_left = self.fleet_dx < 0 && left <= 1;
                if hit_right || hit_left {
                    self.fleet_y += 1;
                    self.fleet_dx = -self.fleet_dx;
                } else {
                    self.fleet_x += self.fleet_dx;
                }
            }
        }

        // Check fleet reached player
        let lowest_row = (0..ALIEN_ROWS)
            .rev()
            .find(|&r| self.alive[r].iter().any(|&a| a));
        if let Some(lr) = lowest_row {
            let (_, ly) = self.alien_pos(lr, 0);
            if ly >= self.player_y() {
                self.game_over = true;
                return;
            }
        }

        // Move player bullet
        if let Some((bx, by)) = self.player_bullet {
            let nby = by - 1;
            if nby < 1 || self.damage_shield(bx, nby) {
                self.player_bullet = None;
            } else {
                // Check vs aliens
                let mut hit = false;
                'check: for (r, pts) in ALIEN_PTS.iter().enumerate() {
                    for c in 0..ALIEN_COLS {
                        if !self.alive[r][c] {
                            continue;
                        }
                        let (ax, ay) = self.alien_pos(r, c);
                        if bx >= ax - 1 && bx <= ax + 1 && nby >= ay && nby <= ay + 1 {
                            self.alive[r][c] = false;
                            self.score += pts;
                            self.explosions.push(Explosion {
                                x: ax,
                                y: ay,
                                ticks: 8,
                            });
                            hit = true;
                            // Speed up fleet
                            let cnt = self.alive_count();
                            self.move_interval = (1 + cnt as u32 / 3).max(1);
                            break 'check;
                        }
                    }
                }
                if hit {
                    self.player_bullet = None;
                    if self.alive_count() == 0 {
                        self.cleared = true;
                        return;
                    }
                } else {
                    self.player_bullet = Some((bx, nby));
                }
            }
        }

        // Alien fire
        let fire_interval = 25u32.saturating_sub(self.level * 2).max(6);
        self.alien_fire_accum += 1;
        if self.alien_fire_accum >= fire_interval {
            self.alien_fire_accum = 0;
            // Bottom-most alien per random column
            let mut candidates: Vec<(usize, usize)> = Vec::new();
            for c in 0..ALIEN_COLS {
                for r in (0..ALIEN_ROWS).rev() {
                    if self.alive[r][c] {
                        candidates.push((r, c));
                        break;
                    }
                }
            }
            if !candidates.is_empty() {
                let i = (self.lcg_next() as usize) % candidates.len();
                let (r, c) = candidates[i];
                let (ax, ay) = self.alien_pos(r, c);
                self.alien_bullets.push((ax, ay + 1));
            }
        }

        // Move alien bullets (every 2 ticks)
        let rows = self.rows as i32;
        let player_y = self.player_y();
        let player_x = self.player_x;
        let mut i = 0;
        while i < self.alien_bullets.len() {
            let (bx, by) = self.alien_bullets[i];
            let nby = by + 1;
            if nby >= rows {
                self.alien_bullets.remove(i);
                continue;
            }
            if self.damage_shield(bx, nby) {
                self.alien_bullets.remove(i);
                continue;
            }
            if nby == player_y && bx >= player_x - 2 && bx <= player_x + 2 {
                self.alien_bullets.remove(i);
                self.lives = self.lives.saturating_sub(1);
                if self.lives == 0 {
                    self.game_over = true;
                    return;
                }
                self.player_bullet = None;
                self.alien_bullets.clear();
                break;
            }
            self.alien_bullets[i] = (bx, nby);
            i += 1;
        }
    }

    fn shoot(&mut self) {
        if self.player_bullet.is_none() {
            let py = self.player_y();
            self.player_bullet = Some((self.player_x, py - 1));
        }
    }

    fn move_player(&mut self, dx: i32) {
        self.player_x = (self.player_x + dx).max(2).min(self.cols as i32 - 3);
    }
}

fn draw(f: &mut Frame, game: &Game) {
    let area = f.area();

    f.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    // HUD
    let lives_str: String = "♥ ".repeat(game.lives as usize);
    let hud = format!(
        " SPACE INVADERS  Lv{}  Score: {}  {}  [←→] Move  [Space] Fire  [p] Pause  [r] Restart  [q] Quit",
        game.level, game.score, lives_str.trim_end()
    );
    f.render_widget(
        Paragraph::new(hud).style(Style::default().bg(Color::Rgb(15, 0, 30)).fg(Color::White)),
        Rect::new(area.x, area.y, area.width, 1),
    );

    if game.paused && !game.game_over {
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

    if game.cleared {
        let lines = vec![
            Line::from(Span::styled(
                "LEVEL CLEAR!",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "Space / Enter = next level   r = restart   q = quit",
                Style::default().fg(Color::White),
            )),
        ];
        f.render_widget(
            Paragraph::new(lines)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Rgb(15, 0, 30))),
            Rect::new(area.x, area.y + area.height / 2 - 2, area.width, 4),
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

    // Alien chars and colours per row
    const ALIEN_CH: [char; ALIEN_ROWS] = ['▲', '◆', '●'];
    const ALIEN_FG: [Color; ALIEN_ROWS] = [
        Color::Rgb(255, 100, 100),
        Color::Rgb(100, 255, 100),
        Color::Rgb(100, 180, 255),
    ];

    for r in 0..ALIEN_ROWS {
        for c in 0..ALIEN_COLS {
            if !game.alive[r][c] {
                continue;
            }
            let (ax, ay) = game.alien_pos(r, c);
            if ax < 0 || ay < 1 || ax >= area.width as i32 || ay >= area.height as i32 {
                continue;
            }
            if let Some(cell) = buf.cell_mut((ax as u16, ay as u16)) {
                cell.set_char(ALIEN_CH[r]).set_fg(ALIEN_FG[r]);
            }
        }
    }

    // Explosions
    for e in &game.explosions {
        let ch = if e.ticks > 4 { '*' } else { '+' };
        for dy in 0..2i32 {
            let y = e.y + dy;
            if y < 1 || y >= area.height as i32 {
                continue;
            }
            for dx in -1..=1i32 {
                let x = e.x + dx;
                if x < 0 || x >= area.width as i32 {
                    continue;
                }
                if let Some(cell) = buf.cell_mut((x as u16, y as u16)) {
                    cell.set_char(ch).set_fg(Color::Yellow);
                }
            }
        }
    }

    // Shields
    let sy = game.shield_y();
    for s in 0..SHIELD_COUNT {
        let sx = game.shield_x(s);
        for ry in 0..SHIELD_H {
            for rx in 0..SHIELD_W {
                let hp = game.shields[s][ry][rx];
                if hp == 0 {
                    continue;
                }
                let ch = match hp {
                    4 => '█',
                    3 => '▓',
                    2 => '▒',
                    1 => '░',
                    _ => continue,
                };
                let x = sx + rx as i32;
                let y = sy + ry as i32;
                if x >= 0 && y > 0 && (x as u16) < area.width && (y as u16) < area.height {
                    if let Some(cell) = buf.cell_mut((x as u16, y as u16)) {
                        cell.set_char(ch).set_fg(Color::Rgb(100, 220, 100));
                    }
                }
            }
        }
    }

    // Player
    let py = game.player_y();
    if py > 0 && (py as u16) < area.height {
        let ship = [(-2, '▗'), (-1, '█'), (0, '▲'), (1, '█'), (2, '▖')];
        for (dx, ch) in ship {
            let x = game.player_x + dx;
            if x >= 0 && (x as u16) < area.width {
                if let Some(cell) = buf.cell_mut((x as u16, py as u16)) {
                    cell.set_char(ch).set_fg(Color::Rgb(100, 220, 255));
                }
            }
        }
    }

    // Player bullet
    if let Some((bx, by)) = game.player_bullet {
        if bx >= 0 && by > 0 && (bx as u16) < area.width && (by as u16) < area.height {
            if let Some(cell) = buf.cell_mut((bx as u16, by as u16)) {
                cell.set_char('|').set_fg(Color::Yellow);
            }
        }
    }

    // Alien bullets
    for &(bx, by) in &game.alien_bullets {
        if bx >= 0 && by > 0 && (bx as u16) < area.width && (by as u16) < area.height {
            if let Some(cell) = buf.cell_mut((bx as u16, by as u16)) {
                cell.set_char('!').set_fg(Color::Rgb(255, 80, 80));
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
                    KeyCode::Char('p') => game.paused = !game.paused,
                    KeyCode::Left => game.move_player(-2),
                    KeyCode::Right => game.move_player(2),
                    KeyCode::Char(' ') | KeyCode::Enter if game.cleared => game.next_level(),
                    KeyCode::Char(' ') => game.shoot(),
                    _ => {}
                },
                Event::Resize(w, h) => {
                    game.cols = w;
                    game.rows = h;
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
