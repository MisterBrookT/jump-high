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

struct Card { q_en: &'static str, a_en: &'static str, q_zh: &'static str, a_zh: &'static str }

const CARDS: &[Card] = &[
    Card { q_en: "What does the 'B' in Benoit B. Mandelbrot stand for?", a_en: "Benoit B. Mandelbrot — it's a recursive joke. There is no middle name.", q_zh: "Benoit B. Mandelbrot 的 B 代表什么？", a_zh: "Benoit B. Mandelbrot —— 一个递归笑话，他没有中间名。" },
    Card { q_en: "Why is a byte 8 bits, not some other number?", a_en: "IBM System/360 (1964) standardized 8-bit bytes to encode one character in EBCDIC. Before that, bytes ranged from 5 to 9 bits.", q_zh: "为什么一个字节是 8 位而不是其他数？", a_zh: "IBM System/360（1964）将 8 位标准化为一个字节，以编码 EBCDIC 字符。此前字节从 5 到 9 位不等。" },
    Card { q_en: "What is the origin of the word 'algorithm'?", a_en: "From al-Khwārizmī, a 9th-century Persian mathematician whose name referred to his birthplace Khwarezm (modern Uzbekistan).", q_zh: "\u{201c}算法\u{201d}(algorithm) 一词的来源是什么？", a_zh: "源自 al-Khwārizmī，9 世纪波斯数学家，名字来自其出生地花剌子模（今乌兹别克斯坦）。" },
    Card { q_en: "How many possible games of chess exist (Shannon number)?", a_en: "~10^120 possible games — more than atoms in the observable universe (~10^80).", q_zh: "国际象棋有多少种可能的棋局（香农数）？", a_zh: "约 10^120 种可能棋局 —— 比可观测宇宙中的原子数（~10^80）还多。" },
    Card { q_en: "What is the fastest algorithm for matrix multiplication?", a_en: "As of 2024, the best known is O(n^2.371552) by Williams, Xu, Xu & Zhou (2024), improving on Strassen's O(n^2.807) from 1969.", q_zh: "目前最快的矩阵乘法算法复杂度是多少？", a_zh: "截至 2024 年，最优为 O(n^2.371552)（Williams, Xu, Xu & Zhou, 2024），改进了 Strassen 1969 年的 O(n^2.807)。" },
    Card { q_en: "Why do we say 'bugs' in software?", a_en: "Grace Hopper's team found a moth in a Mark II relay in 1947 and taped it in the logbook. The term predates this — Edison used 'bug' for defects in 1878.", q_zh: "为什么软件缺陷叫 \u{201c}bug\u{201d}？", a_zh: "1947 年 Grace Hopper 团队在 Mark II 继电器中发现一只飞蛾并贴在日志上。但该词更早——爱迪生 1878 年就用 bug 指代缺陷。" },
    Card { q_en: "What is the Banach–Tarski paradox?", a_en: "A solid ball in 3D can be decomposed into finitely many pieces and reassembled into two identical copies of the original ball, using only rotations and translations.", q_zh: "什么是巴拿赫-塔斯基悖论？", a_zh: "一个三维实心球可以被分成有限块，仅通过旋转和平移重新组装成两个与原球完全相同的球。" },
    Card { q_en: "How far does light travel in one nanosecond?", a_en: "About 30 cm (roughly one foot). Grace Hopper famously handed out 'nanosecond' wires of that length.", q_zh: "光在一纳秒内传播多远？", a_zh: "约 30 厘米。Grace Hopper 曾分发这个长度的电线来直观展示\u{201c}一纳秒\u{201d}。" },
    Card { q_en: "What is the etymology of 'quarantine'?", a_en: "From Italian 'quaranta giorni' (40 days) — the period ships waited offshore during the Black Death in 14th-century Venice.", q_zh: "\u{201c}隔离\u{201d}(quarantine) 的词源是什么？", a_zh: "源自意大利语 quaranta giorni（40 天）—— 14 世纪威尼斯黑死病期间船只在海上等待的天数。" },
    Card { q_en: "What is the halting problem?", a_en: "Turing proved in 1936 that no general algorithm can decide whether an arbitrary program will eventually halt or run forever.", q_zh: "什么是停机问题？", a_zh: "图灵于 1936 年证明：不存在通用算法能判定任意程序是否会最终停止。" },
    Card { q_en: "Why is the sky blue?", a_en: "Rayleigh scattering: shorter (blue) wavelengths scatter ~16× more than red in the atmosphere. At sunset the path is longer, so blue scatters away and red dominates.", q_zh: "天空为什么是蓝色的？", a_zh: "瑞利散射：蓝色短波在大气中散射强度约为红色的 16 倍。日落时光程更长，蓝光散尽，红色主导。" },
    Card { q_en: "What was the first message sent over ARPANET?", a_en: "'LO' — they tried to type 'LOGIN' on Oct 29, 1969, but the system crashed after two characters.", q_zh: "ARPANET 上发送的第一条消息是什么？", a_zh: "\u{201c}LO\u{201d} —— 1969 年 10 月 29 日他们试图输入 LOGIN，但系统在两个字符后崩溃了。" },
    Card { q_en: "How many transistors are in Apple's M4 chip?", a_en: "~28 billion transistors on a 3nm process (2024). The first Intel 4004 (1971) had 2,300.", q_zh: "Apple M4 芯片有多少个晶体管？", a_zh: "约 280 亿个晶体管，3nm 工艺（2024）。第一颗 Intel 4004（1971）仅有 2300 个。" },
    Card { q_en: "What is the oldest known written language?", a_en: "Sumerian cuneiform, from ~3400 BCE in Mesopotamia. It was used for over 3,000 years.", q_zh: "已知最古老的书写文字是什么？", a_zh: "苏美尔楔形文字，约公元前 3400 年出现于美索不达米亚，使用超过 3000 年。" },
    Card { q_en: "What is a quine in programming?", a_en: "A program that outputs its own source code without reading any input. Named after logician Willard Van Orman Quine.", q_zh: "编程中什么是 quine？", a_zh: "一个不读取任何输入、输出自身源代码的程序。以逻辑学家 Willard Van Orman Quine 命名。" },
    Card { q_en: "How old is the universe?", a_en: "~13.8 billion years, determined from the cosmic microwave background (Planck satellite, 2018 final results).", q_zh: "宇宙有多大年龄？", a_zh: "约 138 亿年，由宇宙微波背景辐射测定（普朗克卫星 2018 年最终结果）。" },
];

const AMBER: Color = Color::Rgb(245, 160, 50);
const CREAM: Color = Color::Rgb(255, 248, 230);
const DIM: Color = Color::Rgb(180, 170, 150);
const BG: Color = Color::Rgb(30, 28, 26);

fn read_lang() -> &'static str {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = format!("{home}/.config/paws/lang");
    match std::fs::read_to_string(&path) {
        Ok(s) => match s.trim() {
            "zh" => "zh",
            _ => "en",
        },
        Err(_) => "en",
    }
}

fn center(area: Rect, w: u16, h: u16) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

fn shuffle(order: &mut Vec<usize>) {
    // Fisher-Yates using simple xorshift
    let mut seed: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    for i in (1..order.len()).rev() {
        seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;
        let j = (seed as usize) % (i + 1);
        order.swap(i, j);
    }
}

fn main() -> io::Result<()> {
    let lang = read_lang();
    let mut order: Vec<usize> = (0..CARDS.len()).collect();
    shuffle(&mut order);

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut idx: usize = 0;
    let mut revealed = false;

    loop {
        term.draw(|f| {
            let area = f.area();
            f.render_widget(Block::default().style(Style::default().bg(BG)), area);

            let card = &CARDS[order[idx]];
            let (q, a) = if lang == "zh" { (card.q_zh, card.a_zh) } else { (card.q_en, card.a_en) };
            let header = if lang == "zh" { "知识 · 每天一点点" } else { "Knowledge · a little every day" };
            let counter = format!("{}/{}", idx + 1, CARDS.len());
            let hint = if lang == "zh" {
                if revealed { "n/→ 下一张  p/← 上一张  r 重洗  q 退出" } else { "空格/回车 揭晓答案" }
            } else {
                if revealed { "n/→ next  p/← prev  r reshuffle  q quit" } else { "SPACE/ENTER to reveal" }
            };

            let box_w = 60u16.min(area.width.saturating_sub(4));
            let box_h = 18u16.min(area.height.saturating_sub(2));
            let box_area = center(area, box_w, box_h);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AMBER))
                .style(Style::default().bg(BG));
            let inner = block.inner(box_area);
            f.render_widget(block, box_area);

            let chunks = Layout::vertical([
                Constraint::Length(2), // header + counter
                Constraint::Min(3),   // question
                Constraint::Length(4), // answer area
                Constraint::Length(1), // hint
            ]).split(inner);

            // Header
            let hdr_line = Line::from(vec![
                Span::styled(header, Style::default().fg(AMBER).add_modifier(Modifier::ITALIC)),
                Span::raw("  "),
                Span::styled(&counter, Style::default().fg(DIM)),
            ]);
            f.render_widget(Paragraph::new(hdr_line).centered(), chunks[0]);

            // Question
            let q_para = Paragraph::new(q)
                .style(Style::default().fg(CREAM).add_modifier(Modifier::BOLD))
                .wrap(Wrap { trim: true })
                .centered();
            f.render_widget(q_para, chunks[1]);

            // Answer
            if revealed {
                let divider = Line::from(Span::styled("───", Style::default().fg(DIM)));
                let ans = Paragraph::new(vec![divider, Line::raw(""), Line::styled(a, Style::default().fg(DIM))])
                    .wrap(Wrap { trim: true })
                    .centered();
                f.render_widget(ans, chunks[2]);
            }

            // Hint
            let hint_line = Line::from(Span::styled(hint, Style::default().fg(DIM)));
            f.render_widget(Paragraph::new(hint_line).centered(), chunks[3]);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char(' ') | KeyCode::Enter => { revealed = true; }
                    KeyCode::Char('n') | KeyCode::Right => {
                        idx = (idx + 1) % CARDS.len();
                        revealed = false;
                    }
                    KeyCode::Char('p') | KeyCode::Left => {
                        idx = if idx == 0 { CARDS.len() - 1 } else { idx - 1 };
                        revealed = false;
                    }
                    KeyCode::Char('r') => {
                        shuffle(&mut order);
                        idx = 0;
                        revealed = false;
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
