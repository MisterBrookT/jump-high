use std::io;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

#[derive(Clone, Copy)]
enum Lang { En, Zh, Ja, Ko }
fn current_lang() -> Lang {
    let p = std::env::var("HOME").map(|h| format!("{h}/.config/paws/lang")).unwrap_or_default();
    match std::fs::read_to_string(&p).unwrap_or_default().trim() {
        "zh" => Lang::Zh, "ja" => Lang::Ja, "ko" => Lang::Ko, _ => Lang::En,
    }
}

struct Quest {
    emoji: &'static str,
    title: [&'static str; 4],      // En, Zh, Ja, Ko
    desc: [&'static [&'static str]; 4],
    time: [&'static str; 4],
}

impl Quest {
    fn t_title(&self, lang: Lang) -> &'static str {
        self.title[lang_idx(lang)]
    }
    fn t_desc(&self, lang: Lang) -> &'static [&'static str] {
        self.desc[lang_idx(lang)]
    }
    fn t_time(&self, lang: Lang) -> &'static str {
        self.time[lang_idx(lang)]
    }
}

fn lang_idx(lang: Lang) -> usize {
    match lang {
        Lang::En => 0,
        Lang::Zh => 1,
        Lang::Ja => 2,
        Lang::Ko => 3,
    }
}

const QUESTS: &[Quest] = &[
    Quest {
        emoji: "🚶",
        title: ["Take a Walk Downstairs", "去楼下走走", "外に出て歩こう", "밖에 나가 걸어보자"],
        desc: [
            &[
                "Put down the keyboard, go downstairs and walk around the block.",
                "No phone — just walk, feel your steps and breath.",
            ],
            &[
                "放下键盘，下楼绕着小区或街区走一圈。",
                "不看手机，只是走，感受脚步和呼吸。",
            ],
            &[
                "キーボードを置いて、外に出て近所を一周歩こう。",
                "スマホは見ない。ただ歩いて、足取りと呼吸を感じよう。",
            ],
            &[
                "키보드를 내려놓고, 밖에 나가 동네를 한 바퀴 걸어보자.",
                "폰은 보지 말고, 그냥 걸으며 발걸음과 호흡을 느껴보자.",
            ],
        ],
        time: ["10 min", "10 分钟", "10 分", "10 분"],
    },
    Quest {
        emoji: "📞",
        title: ["Call a Good Friend", "给好朋友打个电话", "友達に電話しよう", "좋은 친구에게 전화하자"],
        desc: [
            &[
                "Think of someone you miss but haven't reached out to.",
                "Call them — no reason needed, just ask \"how've you been?\"",
            ],
            &[
                "想一个最近想念、但没联系的人。",
                "打过去，不为什么，就问问「最近怎么样」。",
            ],
            &[
                "最近会いたいけど連絡していない人を思い浮かべよう。",
                "電話して、理由はいらない。「最近どう？」って聞くだけ。",
            ],
            &[
                "최근 보고 싶지만 연락 못 한 사람을 떠올려보자.",
                "전화해서, 이유 없이 그냥 \"요즘 어때?\"라고 물어보자.",
            ],
        ],
        time: ["10 min", "10 分钟", "10 分", "10 분"],
    },
    Quest {
        emoji: "🤸",
        title: ["Move Your Body", "动一动身体", "体を動かそう", "몸을 움직이자"],
        desc: [
            &[
                "Wall Angels ×15: back against wall, slide arms up & down — fixes rounded shoulders",
                "Chin Tucks ×10: retract chin horizontally (make a double chin) — counters forward head",
                "Plank ×3 sets, 30s each: brace your core, don't let hips sag",
                "Push-ups ×10: hands shoulder-width, lower until chest nearly touches floor",
            ],
            &[
                "靠墙天使 ×15：背贴墙，手臂贴墙上下滑动，专治圆肩",
                "收下巴 ×10：下巴水平后缩（像双下巴），对抗头前倾",
                "平板支撑 ×3组，每组 30 秒：收紧核心，别塌腰",
                "俯卧撑 ×10：手与肩同宽，下到胸贴近地面",
            ],
            &[
                "ウォールエンジェル ×15：壁に背をつけ、腕を壁に沿って上下 — 巻き肩改善",
                "チンタック ×10：顎を水平に引く（二重顎を作る感じ）— 前傾対策",
                "プランク ×3セット、各30秒：体幹を締めて、腰を落とさない",
                "腕立て伏せ ×10：手は肩幅、胸が床に近づくまで下ろす",
            ],
            &[
                "월 엔젤 ×15: 벽에 등을 대고 팔을 위아래로 — 굽은 어깨 교정",
                "턱 당기기 ×10: 턱을 수평으로 당기기(이중턱 만들기) — 거북목 대응",
                "플랭크 ×3세트, 각 30초: 코어 조이고, 허리 처지지 않게",
                "푸시업 ×10: 손은 어깨 너비, 가슴이 바닥에 닿을 때까지",
            ],
        ],
        time: ["10 min", "10 分钟", "10 分", "10 분"],
    },
    Quest {
        emoji: "🍑",
        title: ["Treat Yourself", "犒劳一下自己", "自分にご褒美", "나를 위한 간식"],
        desc: [
            &[
                "Go out and buy some fruit or snack you've been craving.",
                "Eat it slowly when you're back — not in front of a screen.",
            ],
            &[
                "出门买点你最近想吃的水果或小零食。",
                "回来慢慢吃，不要边吃边看屏幕。",
            ],
            &[
                "最近食べたかった果物やお菓子を買いに行こう。",
                "戻ったらゆっくり食べよう。画面を見ながらはダメ。",
            ],
            &[
                "요즘 먹고 싶었던 과일이나 간식을 사러 나가자.",
                "돌아와서 천천히 먹자. 화면 보면서 먹지 말고.",
            ],
        ],
        time: ["15 min", "15 分钟", "15 分", "15 분"],
    },
    Quest {
        emoji: "🧘",
        title: ["Sit Still for Five Minutes", "静坐五分钟", "5分間静かに座ろう", "5분간 가만히 앉자"],
        desc: [
            &[
                "Find a quiet spot, sit down, close your eyes.",
                "Inhale 4s, exhale 6s; when your mind wanders, gently bring focus back to breath.",
            ],
            &[
                "找个安静的地方坐下，闭眼。",
                "吸气 4 秒，呼气 6 秒；走神了就轻轻把注意力带回呼吸。",
            ],
            &[
                "静かな場所に座って、目を閉じよう。",
                "4秒吸って、6秒吐く。気が散ったら、そっと呼吸に意識を戻そう。",
            ],
            &[
                "조용한 곳에 앉아서 눈을 감자.",
                "4초 들이쉬고, 6초 내쉬기. 딴생각이 나면 부드럽게 호흡으로 돌아오자.",
            ],
        ],
        time: ["5 min", "5 分钟", "5 分", "5 분"],
    },
    Quest {
        emoji: "🎬",
        title: ["Record a Gratitude Clip", "录一段感谢", "感謝を録画しよう", "감사 영상을 찍자"],
        desc: [
            &[
                "Open your phone camera, face yourself: who are you grateful for lately? Why?",
                "You don't have to send it — keep it for yourself.",
            ],
            &[
                "打开手机摄像头，对着自己说：最近想感谢谁？为什么？",
                "录完不用发，留给自己看。",
            ],
            &[
                "スマホのカメラを開いて自分に向けて：最近感謝したい人は？なぜ？",
                "送らなくていい。自分のために残そう。",
            ],
            &[
                "폰 카메라를 켜고 자신을 향해: 최근 감사한 사람은? 왜?",
                "보내지 않아도 돼. 나를 위해 남겨두자.",
            ],
        ],
        time: ["5 min", "5 分钟", "5 分", "5 분"],
    },
];

fn pseudo_random(seed: u64) -> usize {
    let h = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (h >> 33) as usize % QUESTS.len()
}

fn pick_quest() -> usize {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    pseudo_random(now.as_secs() ^ now.subsec_nanos() as u64)
}

fn controls_line(lang: Lang) -> Vec<Span<'static>> {
    let (reroll, pause, quit) = match lang {
        Lang::En => ("reroll", "pause", "quit"),
        Lang::Zh => ("换一个", "暂停", "退出"),
        Lang::Ja => ("別のへ", "一時停止", "終了"),
        Lang::Ko => ("다시", "일시정지", "종료"),
    };
    let key_style = Style::default().fg(Color::Rgb(180, 140, 200)).add_modifier(Modifier::BOLD);
    let txt_style = Style::default().fg(Color::Rgb(100, 100, 120));
    vec![
        Span::styled("r", key_style),
        Span::styled(format!(" {}  ", reroll), txt_style),
        Span::styled("p", key_style),
        Span::styled(format!(" {}  ", pause), txt_style),
        Span::styled("q", key_style),
        Span::styled(format!(" {}", quit), txt_style),
    ]
}

fn box_title(lang: Lang) -> &'static str {
    match lang {
        Lang::En => " 🌍 Earth Online · Side Quest ",
        Lang::Zh => " 🌍 地球Online · Side Quest ",
        Lang::Ja => " 🌍 地球オンライン · サイドクエスト ",
        Lang::Ko => " 🌍 지구 온라인 · 사이드 퀘스트 ",
    }
}

fn paused_label(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "⏸  paused",
        Lang::Zh => "⏸  已暂停",
        Lang::Ja => "⏸  一時停止中",
        Lang::Ko => "⏸  일시정지",
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let lang = current_lang();
    let mut quest_idx = pick_quest();
    let mut start = Instant::now();
    let mut paused = false;
    let mut elapsed_when_paused = Duration::ZERO;

    loop {
        let elapsed = if paused {
            elapsed_when_paused
        } else {
            elapsed_when_paused + start.elapsed()
        };

        terminal.draw(|f| {
            let quest = &QUESTS[quest_idx];
            let area = f.area();

            f.render_widget(
                Block::default().style(Style::default().bg(Color::Rgb(20, 20, 30))),
                area,
            );

            let desc_lines = quest.t_desc(lang);
            // box height = 3 (border top + title + blank) + desc_lines + 1 (time) + 1 (border bottom)
            let box_h = (desc_lines.len() as u16) + 5;
            let content_height = box_h + 6; // box + spacers + timer + pause + controls
            let content_width = 64u16.min(area.width.saturating_sub(4));
            let cx = area.x + area.width.saturating_sub(content_width) / 2;
            let cy = area.y + area.height.saturating_sub(content_height) / 2;
            let content_area = Rect::new(cx, cy, content_width, content_height);

            let chunks = Layout::vertical([
                Constraint::Length(box_h),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(content_area);

            // Quest box
            let mut para_lines = vec![
                Line::raw(""),
                Line::from(Span::styled(
                    format!("  {} {} ", quest.emoji, quest.t_title(lang)),
                    Style::default().fg(Color::Rgb(255, 230, 180)).add_modifier(Modifier::BOLD),
                )),
                Line::raw(""),
            ];
            for dl in desc_lines {
                para_lines.push(Line::from(Span::styled(
                    *dl,
                    Style::default().fg(Color::Rgb(180, 180, 200)),
                )));
            }
            para_lines.push(Line::from(Span::styled(
                format!("⏱  {}", quest.t_time(lang)),
                Style::default().fg(Color::Rgb(130, 160, 180)),
            )));

            let quest_block = Paragraph::new(para_lines)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Rgb(80, 80, 120)))
                        .title(box_title(lang))
                        .title_style(Style::default().fg(Color::Rgb(120, 180, 140))),
                );
            f.render_widget(quest_block, chunks[0]);

            // Timer
            let secs_total = elapsed.as_secs();
            let timer = Paragraph::new(Line::from(Span::styled(
                format!("{}:{:02}", secs_total / 60, secs_total % 60),
                Style::default().fg(Color::Rgb(100, 140, 160)),
            )))
            .alignment(Alignment::Center);
            f.render_widget(timer, chunks[2]);

            // Pause indicator
            if paused {
                let p = Paragraph::new(Line::from(Span::styled(
                    paused_label(lang),
                    Style::default().fg(Color::Rgb(200, 160, 80)),
                )))
                .alignment(Alignment::Center);
                f.render_widget(p, chunks[4]);
            }

            // Controls
            let ctrl = Paragraph::new(Line::from(controls_line(lang))).alignment(Alignment::Center);
            f.render_widget(ctrl, chunks[6]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => {
                        let old = quest_idx;
                        quest_idx = pick_quest();
                        if quest_idx == old {
                            quest_idx = (quest_idx + 1) % QUESTS.len();
                        }
                        elapsed_when_paused = Duration::ZERO;
                        start = Instant::now();
                        paused = false;
                    }
                    KeyCode::Char('p') => {
                        if paused {
                            start = Instant::now();
                            paused = false;
                        } else {
                            elapsed_when_paused += start.elapsed();
                            paused = true;
                        }
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
