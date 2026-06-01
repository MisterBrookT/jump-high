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
    note: &'static str, // why it's good — a short appreciation
}

// Classic Chinese poems (唐诗宋词) — all public domain.
const POEMS_ZH: &[Poem] = &[
    Poem { title: "静夜思", author: "李白 · 唐", lines: &["床前明月光，疑是地上霜。", "举头望明月，低头思故乡。"],
        note: "二十字、无一生僻字，用「霜」「月」的清冷把游子思乡写到极处。平淡见真情，成了中国人共同的乡愁原型。" },
    Poem { title: "登鹳雀楼", author: "王之涣 · 唐", lines: &["白日依山尽，黄河入海流。", "欲穷千里目，更上一层楼。"],
        note: "前两句写景壮阔，后两句由景入理：把「登高望远」升华为人生哲理，景与理浑然一体，毫不说教。" },
    Poem { title: "春晓", author: "孟浩然 · 唐", lines: &["春眠不觉晓，处处闻啼鸟。", "夜来风雨声，花落知多少。"],
        note: "全从听觉落笔，不写所见写所闻。在惜花的怅惘里，藏着对生命悄然流逝的温柔体察。" },
    Poem { title: "江雪", author: "柳宗元 · 唐", lines: &["千山鸟飞绝，万径人踪灭。", "孤舟蓑笠翁，独钓寒江雪。"],
        note: "「千山」「万径」的空旷反衬一叶孤舟，二十字一幅极简水墨。孤绝中见诗人贬谪后的傲岸风骨。" },
    Poem { title: "相思", author: "王维 · 唐", lines: &["红豆生南国，春来发几枝。", "愿君多采撷，此物最相思。"],
        note: "借红豆寄相思，物小情深。「愿君多采撷」以日常口吻说尽牵挂，含蓄不直露——正是东方的深情。" },
    Poem { title: "鹿柴", author: "王维 · 唐", lines: &["空山不见人，但闻人语响。", "返景入深林，复照青苔上。"],
        note: "以声写静、以光写幽。王维「诗中有画」的代表作，禅意就在那一束返照的明暗之间。" },
    Poem { title: "竹里馆", author: "王维 · 唐", lines: &["独坐幽篁里，弹琴复长啸。", "深林人不知，明月来相照。"],
        note: "独坐、弹琴、长啸，人与明月相照。写尽隐者的孤而不寂——孤独在此是一种自足的境界。" },
    Poem { title: "题西林壁", author: "苏轼 · 宋", lines: &["横看成岭侧成峰，远近高低各不同。", "不识庐山真面目，只缘身在此山中。"],
        note: "从「横看成岭侧成峰」的观察，推出「身在此山中」的普世哲理：看不清，往往因为身在局中。" },
    Poem { title: "望庐山瀑布", author: "李白 · 唐", lines: &["日照香炉生紫烟，遥看瀑布挂前川。", "飞流直下三千尺，疑是银河落九天。"],
        note: "夸张与想象把瀑布写成天上来物，「疑是银河落九天」气象惊人——典型的李白式浪漫与豪气。" },
    Poem { title: "寻隐者不遇", author: "贾岛 · 唐", lines: &["松下问童子，言师采药去。", "只在此山中，云深不知处。"],
        note: "全篇是一问三答的对话。「云深不知处」在寻而不遇的怅惘里，反让隐者的高逸更引人神往。" },
    Poem { title: "登幽州台歌", author: "陈子昂 · 唐", lines: &["前不见古人，后不见来者。", "念天地之悠悠，独怆然而涕下。"],
        note: "不写一草一木，直抒时空中的孤独。苍茫的宇宙意识里，是知识分子的千古之悲。" },
    Poem { title: "水调歌头 (节选)", author: "苏轼 · 宋", lines: &["明月几时有？把酒问青天。", "但愿人长久，千里共婵娟。"],
        note: "把对亲人的思念，升华为对天下离人的祝福。旷达中见深情，是中秋词千古难越的绝唱。" },
];

// Classic English poems — all public domain (excerpts kept short).
const POEMS_EN: &[Poem] = &[
    Poem { title: "Hope is the thing with feathers", author: "Emily Dickinson", lines: &["Hope is the thing with feathers -", "That perches in the soul -", "And sings the tune without the words -", "And never stops - at all -"],
        note: "One sustained metaphor — hope as a small bird, singing wordlessly and never stopping — turns an abstraction into something you can feel perch in you. The dashes leave room to breathe." },
    Poem { title: "The Road Not Taken", author: "Robert Frost", lines: &["Two roads diverged in a wood, and I —", "I took the one less traveled by,", "And that has made all the difference."],
        note: "Famously misread as a hymn to nonconformity. The quiet irony: the roads were 'really about the same.' It's about the story we tell ourselves afterward." },
    Poem { title: "Stopping by Woods on a Snowy Evening", author: "Robert Frost", lines: &["The woods are lovely, dark and deep,", "But I have promises to keep,", "And miles to go before I sleep,", "And miles to go before I sleep."],
        note: "Repeating the last line hypnotically turns a pause in the snow into a meditation on duty against the pull of rest — and, just beneath it, death." },
    Poem { title: "Sonnet 18", author: "William Shakespeare", lines: &["Shall I compare thee to a summer's day?", "Thou art more lovely and more temperate:"],
        note: "The 'turn' is bold: the beloved outlasts summer not in flesh but in this very poem. Art is offered as a stay against time itself." },
    Poem { title: "The Tyger", author: "William Blake", lines: &["Tyger Tyger, burning bright,", "In the forests of the night;", "What immortal hand or eye,", "Could frame thy fearful symmetry?"],
        note: "All questions, no answers. The hammering rhythm and the unanswerable 'who dared?' confront a creator who made both beauty and terror." },
    Poem { title: "I Wandered Lonely as a Cloud", author: "William Wordsworth", lines: &["I wandered lonely as a cloud", "That floats on high o'er vales and hills,", "When all at once I saw a crowd,", "A host, of golden daffodils;"],
        note: "Its real gift comes later: the flowers return in memory as 'the bliss of solitude.' Nature's true present is recollection." },
    Poem { title: "Ozymandias", author: "Percy Bysshe Shelley", lines: &["'My name is Ozymandias, King of Kings;", "Look on my Works, ye Mighty, and despair!'", "Nothing beside remains."],
        note: "A shattered statue still boasting in an empty desert. The irony skewers the vanity of power — time, not the king, has the last word." },
    Poem { title: "She Walks in Beauty", author: "Lord Byron", lines: &["She walks in beauty, like the night", "Of cloudless climes and starry skies;"],
        note: "Beauty imagined as a balance of dark and light — 'cloudless climes and starry skies.' Harmony, not dazzle, is the ideal." },
    Poem { title: "Invictus", author: "William Ernest Henley", lines: &["It matters not how strait the gate,", "How charged with punishments the scroll,", "I am the master of my fate,", "I am the captain of my soul."],
        note: "Written from a hospital bed as he faced amputation. 'Master of my fate' earns its defiance because it is spoken from real suffering." },
    Poem { title: "Bright Star", author: "John Keats", lines: &["Bright star, would I were stedfast as thou art —", "Not in lone splendour hung aloft the night"],
        note: "Keats craves the star's constancy but not its cold isolation. The whole poem lives in that tension between eternity and human warmth." },
    Poem { title: "A Psalm of Life", author: "Henry W. Longfellow", lines: &["Lives of great men all remind us", "We can make our lives sublime,", "And, departing, leave behind us", "Footprints on the sands of time."],
        note: "A plain, rousing call to live so your life leaves a mark — 'footprints on the sands of time.' Victorian optimism at its most quotable." },
    Poem { title: "No Man Is an Island", author: "John Donne", lines: &["No man is an island,", "entire of itself;", "every man is a piece of the continent,", "a part of the main."],
        note: "From a prose meditation, not a poem. Its claim that every death 'diminishes me' is the root of 'for whom the bell tolls.'" },
];

const AMBER: Color = Color::Rgb(245, 160, 50);
const CREAM: Color = Color::Rgb(255, 248, 230);
const DIM: Color = Color::Rgb(170, 162, 148);
const SOFT: Color = Color::Rgb(150, 165, 150);
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
            let divider = if zh { "── 赏析 ──" } else { "── why it's good ──" };
            let counter = format!("{}/{}", idx + 1, poems.len());
            let hint = if zh {
                " n/→ 下一首 · p/← 上一首 · r 换一批 · q 退出 "
            } else {
                " n/→ next · p/← prev · r shuffle · q quit "
            };

            let box_w = 64u16.min(area.width.saturating_sub(4));
            let box_h = (poem.lines.len() as u16 + 13).min(area.height.saturating_sub(2));
            let box_area = center(area, box_w, box_h);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AMBER))
                .style(Style::default().bg(BG));
            let inner = block.inner(box_area);
            f.render_widget(block, box_area);

            let chunks = Layout::vertical([
                Constraint::Length(1), // header + counter
                Constraint::Min(3),    // poem + appreciation
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

            // Body: title, author, poem, then the appreciation below a divider.
            let mut body: Vec<Line> = vec![
                Line::styled(poem.title, Style::default().fg(CREAM).add_modifier(Modifier::BOLD)),
                Line::styled(poem.author, Style::default().fg(DIM).add_modifier(Modifier::ITALIC)),
                Line::raw(""),
            ];
            for l in poem.lines {
                body.push(Line::styled(*l, Style::default().fg(CREAM)));
            }
            body.push(Line::raw(""));
            body.push(Line::styled(divider, Style::default().fg(SOFT)));
            body.push(Line::styled(poem.note, Style::default().fg(SOFT).add_modifier(Modifier::ITALIC)));

            f.render_widget(
                Paragraph::new(body).wrap(Wrap { trim: true }).centered(),
                chunks[1],
            );

            f.render_widget(
                Paragraph::new(Line::styled(hint, Style::default().fg(DIM))).centered(),
                chunks[2],
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
