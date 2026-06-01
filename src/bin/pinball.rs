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

const TICK: Duration = Duration::from_millis(25); // ~40fps
const GRAVITY: f64 = 0.18;
const TABLE_W: u16 = 44;
const TABLE_H: u16 = 50;
const TW: f64 = TABLE_W as f64;
const TH: f64 = TABLE_H as f64;
const FLIPPER_DUR: Duration = Duration::from_millis(150);
const BALL_R: f64 = 0.6;
const BUMPER_R: f64 = 1.5;
const MAX_SPEED: f64 = 5.5;
const LAUNCH_X: f64 = 41.0; // plunger lane on the right
const LANE_WALL_X: f64 = 38.0; // left wall of launch lane

// Colors — Space Cadet palette
const BG: Color = Color::Rgb(10, 20, 60);
const BORDER_C: Color = Color::Rgb(40, 60, 120);
const BUMPER_C: Color = Color::Rgb(245, 160, 50);
const BUMPER_LIT: Color = Color::Rgb(255, 240, 100);
const BALL_C: Color = Color::Rgb(220, 220, 255);
const CYAN_C: Color = Color::Rgb(80, 220, 255);
const FLIPPER_C: Color = Color::Rgb(200, 200, 220);
const HUD_C: Color = Color::Rgb(245, 160, 50);
const DRAIN_C: Color = Color::Rgb(60, 20, 20);

const RANKS: &[&str] = &[
    "Cadet", "Ensign", "Lieutenant", "Captain",
    "Lt Commander", "Commander", "Commodore", "Admiral", "Fleet Admiral",
];

struct Bumper { x: f64, y: f64, hit_timer: u8, lit: bool }
struct Game {
    bx: f64, by: f64, vx: f64, vy: f64,
    score: u32, balls: u8, multiplier: u8, rank: usize,
    launched: bool, paused: bool, over: bool,
    left_flip: Option<Instant>, right_flip: Option<Instant>,
    bumpers: Vec<Bumper>,
    plunger_power: u8, // 0 = not charging, 1..10 = power
    mission_text: &'static str,
}

impl Game {
    fn new() -> Self {
        Self {
            bx: LAUNCH_X, by: TH - 6.0, vx: 0.0, vy: 0.0,
            score: 0, balls: 3, multiplier: 1, rank: 0,
            launched: false, paused: false, over: false,
            left_flip: None, right_flip: None,
            bumpers: vec![
                Bumper { x: 12.0, y: 10.0, hit_timer: 0, lit: false },
                Bumper { x: 22.0, y: 8.0, hit_timer: 0, lit: false },
                Bumper { x: 32.0, y: 11.0, hit_timer: 0, lit: false },
                Bumper { x: 17.0, y: 17.0, hit_timer: 0, lit: false },
                Bumper { x: 27.0, y: 15.0, hit_timer: 0, lit: false },
                Bumper { x: 10.0, y: 24.0, hit_timer: 0, lit: false },
                Bumper { x: 30.0, y: 22.0, hit_timer: 0, lit: false },
            ],
            plunger_power: 0,
            mission_text: "MISSION: Light all bumpers",
        }
    }

    fn launch(&mut self) {
        if !self.launched && !self.over {
            self.plunger_power = (self.plunger_power + 2).min(10);
        }
    }

    fn release_plunger(&mut self) {
        if !self.launched && !self.over && self.plunger_power > 0 {
            let power = self.plunger_power as f64;
            self.vy = -(power * 0.55 + 1.5);
            self.vx = -0.3;
            self.launched = true;
            self.plunger_power = 0;
        }
    }

    fn reset_ball(&mut self) {
        self.launched = false;
        self.bx = LAUNCH_X;
        self.by = TH - 6.0;
        self.vx = 0.0;
        self.vy = 0.0;
        self.plunger_power = 0;
    }

    fn check_mission_complete(&mut self) {
        if self.bumpers.iter().all(|b| b.lit) {
            // Bonus!
            let bonus = 5000 * self.multiplier as u32;
            self.score += bonus;
            self.multiplier = (self.multiplier + 1).min(9);
            if self.rank < RANKS.len() - 1 { self.rank += 1; }
            // Reset bumper lights for next mission
            for b in &mut self.bumpers { b.lit = false; }
            self.mission_text = "MISSION COMPLETE! Light all bumpers";
        }
    }

    fn tick(&mut self) {
        if self.paused || self.over || !self.launched { return; }
        let now = Instant::now();

        // Gravity
        self.vy += GRAVITY;
        self.bx += self.vx;
        self.by += self.vy;

        // Speed cap
        self.vx = self.vx.clamp(-MAX_SPEED, MAX_SPEED);
        self.vy = self.vy.clamp(-MAX_SPEED, MAX_SPEED);

        // Wall collisions — left wall
        if self.bx <= 1.0 { self.bx = 1.0; self.vx = self.vx.abs() * 0.85; }
        // Right outer wall
        if self.bx >= TW - 1.0 { self.bx = TW - 1.0; self.vx = -self.vx.abs() * 0.85; }
        // Top wall
        if self.by <= 1.0 { self.by = 1.0; self.vy = self.vy.abs() * 0.85; }

        // Launch lane left wall — ball in lane can't go left past it until it exits top
        if self.bx > LANE_WALL_X && self.by > 5.0 {
            if self.bx < LANE_WALL_X + 1.0 && self.vx < 0.0 {
                self.bx = LANE_WALL_X + 1.0;
                self.vx = self.vx.abs() * 0.5;
            }
        }

        // Drain check — gap between flippers at bottom
        let flipper_y = TH - 5.0;
        let drain_left = 14.0;
        let drain_right = 24.0;

        if self.by >= TH - 2.0 {
            if self.bx > drain_left && self.bx < drain_right {
                self.balls -= 1;
                if self.balls == 0 { self.over = true; } else { self.reset_ball(); }
                return;
            }
            // Solid bottom outside drain
            self.by = TH - 2.0;
            self.vy = -self.vy.abs() * 0.6;
        }

        // Flipper physics
        let left_active = self.left_flip.map_or(false, |t| now.duration_since(t) < FLIPPER_DUR);
        let right_active = self.right_flip.map_or(false, |t| now.duration_since(t) < FLIPPER_DUR);

        // Left flipper: x 4..14, near flipper_y
        if left_active && self.by >= flipper_y - 1.5 && self.by <= flipper_y + 1.5
            && self.bx >= 4.0 && self.bx <= 14.0 {
            self.vy = -4.5;
            self.vx += 1.2;
        }
        // Right flipper: x 24..34, near flipper_y
        if right_active && self.by >= flipper_y - 1.5 && self.by <= flipper_y + 1.5
            && self.bx >= 24.0 && self.bx <= 34.0 {
            self.vy = -4.5;
            self.vx -= 1.2;
        }

        // Resting flippers — slight bounce
        if !left_active && self.by >= flipper_y && self.by <= flipper_y + 1.0
            && self.bx >= 4.0 && self.bx <= 14.0 {
            self.by = flipper_y - 0.5;
            self.vy = -self.vy.abs() * 0.4;
        }
        if !right_active && self.by >= flipper_y && self.by <= flipper_y + 1.0
            && self.bx >= 24.0 && self.bx <= 34.0 {
            self.by = flipper_y - 0.5;
            self.vy = -self.vy.abs() * 0.4;
        }

        // Bumper collisions
        for b in &mut self.bumpers {
            let dx = self.bx - b.x;
            let dy = self.by - b.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < BALL_R + BUMPER_R && dist > 0.01 {
                let nx = dx / dist;
                let ny = dy / dist;
                self.vx = nx * 3.5;
                self.vy = ny * 3.5;
                self.bx = b.x + nx * (BUMPER_R + BALL_R + 0.2);
                self.by = b.y + ny * (BUMPER_R + BALL_R + 0.2);
                b.hit_timer = 6;
                b.lit = true;
                self.score += 500 * self.multiplier as u32;
            }
        }

        // Decay hit flash
        for b in &mut self.bumpers { if b.hit_timer > 0 { b.hit_timer -= 1; } }

        // Check mission
        self.check_mission_complete();
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut game = Game::new();
    let mut last_tick = Instant::now();
    let mut plunger_release_pending = false;

    loop {
        if event::poll(Duration::from_millis(5))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('r') => { game = Game::new(); }
                    KeyCode::Char('p') => { game.paused = !game.paused; }
                    KeyCode::Char(' ') => {
                        if !game.launched {
                            game.launch();
                            plunger_release_pending = true;
                        }
                    }
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
            // Auto-release plunger after a short delay
            if plunger_release_pending && !game.launched {
                game.release_plunger();
                plunger_release_pending = false;
            }
            game.tick();
            last_tick = Instant::now();
        }

        term.draw(|f| {
            let area = f.area();
            let total_w = TABLE_W + 2;
            let total_h = TABLE_H + 5; // table + HUD
            let play_h = area.height.saturating_sub(1); // keep the last row for the hint
            let ox = area.width.saturating_sub(total_w) / 2;
            let oy = play_h.saturating_sub(total_h) / 2;
            let rect = Rect::new(ox, oy, total_w.min(area.width), total_h.min(play_h));

            let mut lines: Vec<Line> = Vec::new();
            let now = Instant::now();
            let left_up = game.left_flip.map_or(false, |t| now.duration_since(t) < FLIPPER_DUR);
            let right_up = game.right_flip.map_or(false, |t| now.duration_since(t) < FLIPPER_DUR);

            // HUD lines
            let rank_str = RANKS[game.rank];
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" ╔═ 3D PINBALL: SPACE CADET ═╗  RANK: {} ", rank_str),
                    Style::default().fg(CYAN_C).bg(BG),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" SCORE: {:>9}  BALL: {}/3  MULTx{}  {}",
                        game.score, game.balls, game.multiplier,
                        if game.paused { "[PAUSED]" } else if game.over { "[GAME OVER]" } else { "" }),
                    Style::default().fg(HUD_C).bg(BG),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", game.mission_text),
                    Style::default().fg(CYAN_C).bg(BG),
                ),
            ]));

            // Top border
            let top = format!("╔{}╗", "═".repeat(TABLE_W as usize));
            lines.push(Line::from(Span::styled(top, Style::default().fg(BORDER_C).bg(BG))));

            // Render table
            let flipper_row = (TH - 5.0) as i32;
            for row in 0..TABLE_H as i32 {
                let mut spans: Vec<Span> = Vec::new();
                spans.push(Span::styled("║", Style::default().fg(BORDER_C).bg(BG)));

                for col in 0..TABLE_W as i32 {
                    let fx = col as f64;
                    let fy = row as f64;

                    // Ball
                    let is_ball = game.launched
                        && (fx - game.bx).abs() < 1.0
                        && (fy - game.by).abs() < 0.7;

                    // Bumper
                    let mut bumper_idx: Option<usize> = None;
                    for (i, b) in game.bumpers.iter().enumerate() {
                        if (fx - b.x).abs() < 1.8 && (fy - b.y).abs() < 1.2 {
                            bumper_idx = Some(i);
                            break;
                        }
                    }

                    // Launch lane wall
                    let is_lane_wall = col == LANE_WALL_X as i32 && row > 4;
                    // Plunger indicator
                    let is_plunger = col >= LANE_WALL_X as i32 + 1
                        && col <= TABLE_W as i32 - 2
                        && row >= (TH as i32 - 4)
                        && !game.launched;

                    // Flippers
                    let is_left_flip = row == flipper_row && col >= 4 && col <= 13;
                    let is_right_flip = row == flipper_row && col >= 25 && col <= 34;
                    // Drain
                    let is_drain = row >= (TH as i32 - 2) && col > 14 && col < 24;

                    if is_ball {
                        spans.push(Span::styled("●", Style::default().fg(BALL_C).bg(BG)));
                    } else if let Some(i) = bumper_idx {
                        let b = &game.bumpers[i];
                        let c = if b.hit_timer > 0 { BUMPER_LIT }
                                else if b.lit { Color::Rgb(200, 140, 40) }
                                else { BUMPER_C };
                        let ch = if b.lit { "◉" } else { "○" };
                        spans.push(Span::styled(ch, Style::default().fg(c).bg(BG)));
                    } else if is_lane_wall {
                        spans.push(Span::styled("│", Style::default().fg(BORDER_C).bg(BG)));
                    } else if is_plunger {
                        let power_row = TH as i32 - 4 + (10 - game.plunger_power as i32).max(0) / 3;
                        if row >= power_row {
                            spans.push(Span::styled("▓", Style::default().fg(Color::Rgb(200, 60, 60)).bg(BG)));
                        } else {
                            spans.push(Span::styled("░", Style::default().fg(Color::Rgb(60, 40, 40)).bg(BG)));
                        }
                    } else if is_left_flip {
                        let ch = if left_up { "▀" } else { "▄" };
                        spans.push(Span::styled(ch, Style::default().fg(FLIPPER_C).bg(BG)));
                    } else if is_right_flip {
                        let ch = if right_up { "▀" } else { "▄" };
                        spans.push(Span::styled(ch, Style::default().fg(FLIPPER_C).bg(BG)));
                    } else if is_drain {
                        spans.push(Span::styled(" ", Style::default().fg(DRAIN_C).bg(DRAIN_C)));
                    } else {
                        spans.push(Span::styled(" ", Style::default().bg(BG)));
                    }
                }
                spans.push(Span::styled("║", Style::default().fg(BORDER_C).bg(BG)));
                lines.push(Line::from(spans));
            }

            // Bottom border
            let bot = format!("╚{}╝", "═".repeat(TABLE_W as usize));
            lines.push(Line::from(Span::styled(bot, Style::default().fg(BORDER_C).bg(BG))));

            // Controls
            if game.over {
                lines.push(Line::from(Span::styled(
                    format!("  FINAL SCORE: {}  |  R: restart  Q: quit", game.score),
                    Style::default().fg(HUD_C),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "  SPACE:launch  ←/a:left flip  →/l:right flip  P:pause  Q:quit",
                    Style::default().fg(Color::DarkGray),
                )));
            }

            let para = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
            f.render_widget(para, rect);

            // Bottom hint line
            let hint = " SPACE launch · ←/a left flip · →/l right flip · p pause · r restart · q quit ";
            let hint_line = Line::from(hint).style(Style::default().fg(Color::Rgb(205, 200, 190)));
            let hint_rect = Rect::new(0, area.height.saturating_sub(1), area.width, 1);
            f.render_widget(Paragraph::new(hint_line), hint_rect);
        })?;
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
