use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

struct Poem {
    title: &'static str,
    author: &'static str,
    lines: &'static [&'static str],
}

// Classic Chinese poems (唐诗宋词) — all public domain.
const POEMS_ZH: &[Poem] = &[
    Poem { title: "静夜思", author: "李白 · 唐", lines: &["床前明月光，疑是地上霜。", "举头望明月，低头思故乡。"] },
    Poem { title: "登鹳雀楼", author: "王之涣 · 唐", lines: &["白日依山尽，黄河入海流。", "欲穷千里目，更上一层楼。"] },
    Poem { title: "春晓", author: "孟浩然 · 唐", lines: &["春眠不觉晓，处处闻啼鸟。", "夜来风雨声，花落知多少。"] },
    Poem { title: "江雪", author: "柳宗元 · 唐", lines: &["千山鸟飞绝，万径人踪灭。", "孤舟蓑笠翁，独钓寒江雪。"] },
    Poem { title: "相思", author: "王维 · 唐", lines: &["红豆生南国，春来发几枝。", "愿君多采撷，此物最相思。"] },
    Poem { title: "鹿柴", author: "王维 · 唐", lines: &["空山不见人，但闻人语响。", "返景入深林，复照青苔上。"] },
    Poem { title: "竹里馆", author: "王维 · 唐", lines: &["独坐幽篁里，弹琴复长啸。", "深林人不知，明月来相照。"] },
    Poem { title: "题西林壁", author: "苏轼 · 宋", lines: &["横看成岭侧成峰，远近高低各不同。", "不识庐山真面目，只缘身在此山中。"] },
    Poem { title: "望庐山瀑布", author: "李白 · 唐", lines: &["日照香炉生紫烟，遥看瀑布挂前川。", "飞流直下三千尺，疑是银河落九天。"] },
    Poem { title: "寻隐者不遇", author: "贾岛 · 唐", lines: &["松下问童子，言师采药去。", "只在此山中，云深不知处。"] },
    Poem { title: "登幽州台歌", author: "陈子昂 · 唐", lines: &["前不见古人，后不见来者。", "念天地之悠悠，独怆然而涕下。"] },
    Poem { title: "水调歌头 (节选)", author: "苏轼 · 宋", lines: &["明月几时有？把酒问青天。", "但愿人长久，千里共婵娟。"] },
];

// Classic English poems — all public domain (excerpts kept short).
const POEMS_EN: &[Poem] = &[
    Poem { title: "Hope is the thing with feathers", author: "Emily Dickinson", lines: &["Hope is the thing with feathers -", "That perches in the soul -", "And sings the tune without the words -", "And never stops - at all -"] },
    Poem { title: "The Road Not Taken", author: "Robert Frost", lines: &["Two roads diverged in a wood, and I —", "I took the one less traveled by,", "And that has made all the difference."] },
    Poem { title: "Stopping by Woods on a Snowy Evening", author: "Robert Frost", lines: &["The woods are lovely, dark and deep,", "But I have promises to keep,", "And miles to go before I sleep,", "And miles to go before I sleep."] },
    Poem { title: "Sonnet 18", author: "William Shakespeare", lines: &["Shall I compare thee to a summer's day?", "Thou art more lovely and more temperate:"] },
    Poem { title: "The Tyger", author: "William Blake", lines: &["Tyger Tyger, burning bright,", "In the forests of the night;", "What immortal hand or eye,", "Could frame thy fearful symmetry?"] },
    Poem { title: "I Wandered Lonely as a Cloud", author: "William Wordsworth", lines: &["I wandered lonely as a cloud", "That floats on high o'er vales and hills,", "When all at once I saw a crowd,", "A host, of golden daffodils;"] },
    Poem { title: "Ozymandias", author: "Percy Bysshe Shelley", lines: &["'My name is Ozymandias, King of Kings;", "Look on my Works, ye Mighty, and despair!'", "Nothing beside remains."] },
    Poem { title: "She Walks in Beauty", author: "Lord Byron", lines: &["She walks in beauty, like the night", "Of cloudless climes and starry skies;"] },
    Poem { title: "Invictus", author: "William Ernest Henley", lines: &["It matters not how strait the gate,", "How charged with punishments the scroll,", "I am the master of my fate,", "I am the captain of my soul."] },
    Poem { title: "Bright Star", author: "John Keats", lines: &["Bright star, would I were stedfast as thou art —", "Not in lone splendour hung aloft the night"] },
    Poem { title: "A Psalm of Life", author: "Henry W. Longfellow", lines: &["Lives of great men all remind us", "We can make our lives sublime,", "And, departing, leave behind us", "Footprints on the sands of time."] },
    Poem { title: "No Man Is an Island", author: "John Donne", lines: &["No man is an island,", "entire of itself;", "every man is a piece of the continent,", "a part of the main."] },
];

const AMBER: Color = Color::Rgb(245, 160, 50);
const CREAM: Color = Color::Rgb(255, 248, 230);
const DIM: Color = Color::Rgb(180, 170, 150);
const BG: Color = Color::Rgb(30, 28, 26);

fn is_zh() -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    std::fs::read_to_string(format!("{home}/.config/paws/lang"))
        .map(|s| s.trim() == "zh")
        .unwrap_or(false)
}

fn center(area: Rect, w: u16, h: u16) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

fn shuffle(order: &mut [usize]) {
    let mut seed: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    for i in (1..order.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        order.swap(i, (seed as usize) % (i + 1));
    }
}

fn main() -> io::Result<()> {
    let zh = is_zh();
    let poems = if zh { POEMS_ZH } else { POEMS_EN };
    let mut order: Vec<usize> = (0..poems.len()).collect();
    shuffle(&mut order);

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut idx: usize = 0;
    loop {
        term.draw(|f| {
            let area = f.area();
            f.render_widget(Block::default().style(Style::default().bg(BG)), area);

            let poem = &poems[order[idx]];
            let header = if zh { "诗 · 偷得浮生半日闲" } else { "Poetry · a quiet moment" };
            let counter = format!("{}/{}", idx + 1, poems.len());
            let hint = if zh {
                " n/→ 下一首 · p/← 上一首 · r 换一批 · q 退出 "
            } else {
                " n/→ next · p/← prev · r shuffle · q quit "
            };

            let box_w = 60u16.min(area.width.saturating_sub(4));
            let box_h = (poem.lines.len() as u16 + 8).min(area.height.saturating_sub(2));
            let box_area = center(area, box_w, box_h);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AMBER))
                .style(Style::default().bg(BG));
            let inner = block.inner(box_area);
            f.render_widget(block, box_area);

            let chunks = Layout::vertical([
                Constraint::Length(1), // header + counter
                Constraint::Length(1), // title
                Constraint::Length(1), // author
                Constraint::Min(2),    // poem
                Constraint::Length(1), // hint
            ])
            .split(inner);

            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(header, Style::default().fg(AMBER).add_modifier(Modifier::ITALIC)),
                    Span::raw("  "),
                    Span::styled(&counter, Style::default().fg(DIM)),
                ]))
                .centered(),
                chunks[0],
            );
            f.render_widget(
                Paragraph::new(Line::styled(poem.title, Style::default().fg(CREAM).add_modifier(Modifier::BOLD))).centered(),
                chunks[1],
            );
            f.render_widget(
                Paragraph::new(Line::styled(poem.author, Style::default().fg(DIM).add_modifier(Modifier::ITALIC))).centered(),
                chunks[2],
            );
            let body: Vec<Line> = poem
                .lines
                .iter()
                .map(|l| Line::styled(*l, Style::default().fg(CREAM)))
                .collect();
            f.render_widget(Paragraph::new(body).wrap(Wrap { trim: true }).centered(), chunks[3]);
            f.render_widget(
                Paragraph::new(Line::styled(hint, Style::default().fg(DIM))).centered(),
                chunks[4],
            );
        })?;

        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('n') | KeyCode::Right | KeyCode::Char(' ') => {
                        idx = (idx + 1) % poems.len();
                    }
                    KeyCode::Char('p') | KeyCode::Left => {
                        idx = if idx == 0 { poems.len() - 1 } else { idx - 1 };
                    }
                    KeyCode::Char('r') => {
                        shuffle(&mut order);
                        idx = 0;
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
