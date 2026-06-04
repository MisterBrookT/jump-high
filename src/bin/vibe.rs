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

struct Upgrade {
    name: &'static str,
    emoji: &'static str,
    desc: &'static str,
    base_cost: u64,
    rate: u64,
    count: u32,
}

impl Upgrade {
    fn cost(&self) -> u64 {
        let scale = 1.15f64.powi(self.count as i32);
        ((self.base_cost as f64) * scale) as u64
    }
}

struct Game {
    vibes: u64,
    taps: u64,
    upgrades: Vec<Upgrade>,
    selected: usize,
    last_tick: Instant,
    partial: f64,
}

impl Game {
    fn new() -> Self {
        Game {
            vibes: 0,
            taps: 0,
            upgrades: vec![
                Upgrade {
                    name: "Auto-paw",
                    emoji: "🐾",
                    desc: " 1/s",
                    base_cost: 15,
                    rate: 1,
                    count: 0,
                },
                Upgrade {
                    name: "Vibe buddy",
                    emoji: "🐱",
                    desc: " 5/s",
                    base_cost: 100,
                    rate: 5,
                    count: 0,
                },
                Upgrade {
                    name: "Hype engine",
                    emoji: "🚀",
                    desc: "20/s",
                    base_cost: 500,
                    rate: 20,
                    count: 0,
                },
                Upgrade {
                    name: "Vibe reactor",
                    emoji: "⚡",
                    desc: "100/s",
                    base_cost: 3000,
                    rate: 100,
                    count: 0,
                },
                Upgrade {
                    name: "Galaxy brain",
                    emoji: "🌌",
                    desc: "500/s",
                    base_cost: 15000,
                    rate: 500,
                    count: 0,
                },
            ],
            selected: 0,
            last_tick: Instant::now(),
            partial: 0.0,
        }
    }

    fn rate(&self) -> u64 {
        self.upgrades.iter().map(|u| u.rate * u.count as u64).sum()
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;
        self.partial += self.rate() as f64 * dt;
        let earned = self.partial as u64;
        if earned > 0 {
            self.partial -= earned as f64;
            self.vibes += earned;
        }
    }

    fn tap(&mut self) {
        self.vibes += 1;
        self.taps += 1;
    }

    fn buy(&mut self) {
        let cost = self.upgrades[self.selected].cost();
        if self.vibes >= cost {
            self.vibes -= cost;
            self.upgrades[self.selected].count += 1;
        }
    }
}

fn fmt_n(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn draw(f: &mut Frame, g: &Game) {
    let area = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // ── header ──
    f.render_widget(
        Paragraph::new("✨  V I B E  C L I C K E R  ✨")
            .style(Style::new().fg(Color::Magenta).bold())
            .alignment(Alignment::Center),
        header,
    );

    // ── body: left = tap panel, right = upgrades ──
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(body);

    // tap panel
    let rate = g.rate();
    let vibe_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(fmt_n(g.vibes), Style::new().fg(Color::Yellow).bold()),
            Span::styled(" vibes", Style::new().fg(Color::DarkGray)),
        ]),
        Line::from(vec![Span::styled(
            format!("{}/sec", fmt_n(rate)),
            Style::new().fg(Color::Cyan),
        )]),
        Line::from(""),
        Line::from(Span::styled(
            "── SPACE to vibe ──",
            Style::new().fg(Color::Green),
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("{} taps", g.taps),
            Style::new().fg(Color::DarkGray),
        )]),
    ];
    f.render_widget(
        Paragraph::new(vibe_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::RIGHT)
                    .border_style(Style::new().fg(Color::DarkGray)),
            ),
        left,
    );

    // upgrades panel
    let upgrade_lines: Vec<Line> = std::iter::once(Line::from(Span::styled(
        " Upgrades  ↑↓ / Enter ",
        Style::new().fg(Color::DarkGray),
    )))
    .chain(std::iter::once(Line::from("")))
    .chain(g.upgrades.iter().enumerate().map(|(i, u)| {
        let cost = u.cost();
        let sel = i == g.selected;
        let affordable = g.vibes >= cost;
        let (bg, fg) = if sel {
            (Color::DarkGray, Color::White)
        } else {
            (Color::Black, Color::Gray)
        };
        let cost_fg = if affordable {
            Color::Green
        } else {
            Color::DarkGray
        };
        Line::from(vec![
            Span::styled(
                format!(" {} {:<14}", u.emoji, u.name),
                Style::new().fg(fg).bg(bg),
            ),
            Span::styled(
                format!("x{:<3}", u.count),
                Style::new().fg(Color::Cyan).bg(bg),
            ),
            Span::styled(u.desc.to_string(), Style::new().fg(Color::DarkGray).bg(bg)),
            Span::styled(
                format!(" [{}]", fmt_n(cost)),
                Style::new().fg(cost_fg).bg(bg),
            ),
        ])
    }))
    .collect();

    f.render_widget(Paragraph::new(upgrade_lines), right);

    // ── footer ──
    f.render_widget(
        Paragraph::new("Q · Esc  quit")
            .style(Style::new().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        footer,
    );
}

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _guard = Guard;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut game = Game::new();

    loop {
        game.tick();
        terminal.draw(|f| draw(f, &game))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char(' ') => game.tap(),
                    KeyCode::Up | KeyCode::Char('k') => {
                        if game.selected > 0 {
                            game.selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if game.selected + 1 < game.upgrades.len() {
                            game.selected += 1;
                        }
                    }
                    KeyCode::Enter => game.buy(),
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
