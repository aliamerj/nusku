use std::{
    collections::VecDeque,
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use engine::{
    process_cpu::{ProcessCpuInfo, ProcessCpuReader},
    snapshot::Snapshot,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Gauge, Paragraph, Row, Sparkline, Table, TableState,
    },
    Frame, Terminal,
};

const BG: Color = Color::Rgb(8, 10, 14);
const SURFACE: Color = Color::Rgb(14, 18, 24);
const BORDER: Color = Color::Rgb(32, 42, 58);
const DIM: Color = Color::Rgb(55, 65, 85);
const MUTED: Color = Color::Rgb(90, 105, 130);
const FG: Color = Color::Rgb(200, 210, 225);
const ACCENT: Color = Color::Rgb(80, 200, 160);
const AMBER: Color = Color::Rgb(255, 185, 55);
const HOT: Color = Color::Rgb(255, 90, 80);
const CYAN: Color = Color::Rgb(80, 190, 230);
const HEADER_BG: Color = Color::Rgb(18, 24, 34);

// heat color based on % of ONE core (0–100)
fn heat(p: f64) -> Color {
    if p >= 80.0 {
        HOT
    } else if p >= 50.0 {
        AMBER
    } else if p >= 10.0 {
        ACCENT
    } else {
        MUTED
    }
}

const HISTORY: usize = 120;

pub struct App {
    pid: u32,
    cmd: String,
    snapshot: Option<Snapshot>,
    cpu_info: Option<ProcessCpuInfo>,
    table_state: TableState,
    cpu_history: VecDeque<u64>,
    mem_history: VecDeque<u64>,
    uptime: u64,
    total: u64,
    paused: bool,
}

impl App {
    fn new(pid: u32, cmd: String) -> Self {
        let mut ts = TableState::default();
        ts.select(Some(0));
        Self {
            pid,
            cmd,
            snapshot: None,
            cpu_info: None,
            table_state: ts,
            cpu_history: VecDeque::with_capacity(HISTORY),
            mem_history: VecDeque::with_capacity(HISTORY),
            uptime: 0,
            total: 0,
            paused: false,
        }
    }

    fn ingest(&mut self, snap: Snapshot, cpu_info: Option<ProcessCpuInfo>) {
        if self.paused {
            return;
        }

        //  CPU% comes from /proc (ProcessCpuReader), NOT from eBPF samples
        // eBPF perf_event only fires when the process is ON cpu, so it always
        // reads ~100% for a busy process. /proc ticks give the real picture.
        let cpu_pct = cpu_info.as_ref().map(|c| c.total_percent).unwrap_or(0.0);

        push(&mut self.cpu_history, cpu_pct as u64);
        push(&mut self.mem_history, snap.mem.rss_kb);

        self.total += snap.cpu.total_samples;
        self.uptime += 1;
        self.snapshot = Some(snap);
        self.cpu_info = cpu_info;
    }

    fn down(&mut self) {
        let max = self
            .snapshot
            .as_ref()
            .map(|s| s.cpu.frames.len())
            .unwrap_or(0)
            .saturating_sub(1);
        let next = self
            .table_state
            .selected()
            .unwrap_or(0)
            .saturating_add(1)
            .min(max);
        self.table_state.select(Some(next));
    }
    fn up(&mut self) {
        let prev = self.table_state.selected().unwrap_or(0).saturating_sub(1);
        self.table_state.select(Some(prev));
    }

    fn total_cpu_pct(&self) -> f64 {
        self.cpu_info
            .as_ref()
            .map(|c| c.total_percent)
            .unwrap_or(0.0)
    }
}

fn push(dq: &mut VecDeque<u64>, v: u64) {
    if dq.len() >= HISTORY {
        dq.pop_front();
    }
    dq.push_back(v);
}

// entry point

pub fn run_tui(
    pid: u32,
    cmd: String,
    rx: mpsc::Receiver<Snapshot>,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;
    let mut app = App::new(pid, cmd);

    // ProcessCpuReader reads /proc — this is our real CPU% source
    let mut cpu_reader = ProcessCpuReader::new(pid);

    let tick = Duration::from_millis(100);
    let mut last = Instant::now();

    let result = (|| -> Result<()> {
        loop {
            // Drain all pending snapshots, take most recent
            let mut latest = None;
            while let Ok(s) = rx.try_recv() {
                latest = Some(s);
            }

            if let Some(snap) = latest {
                // Read /proc CPU at same time as snapshot arrives
                let cpu_info = cpu_reader.read();
                app.ingest(snap, cpu_info);
            }

            term.draw(|f| draw(f, &mut app))?;

            let timeout = tick.checked_sub(last.elapsed()).unwrap_or_default();
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            stop.store(true, Ordering::SeqCst);
                            break;
                        }
                        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => app.down(),
                        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => app.up(),
                        (KeyCode::Char('p'), _) => app.paused = !app.paused,
                        _ => {}
                    }
                }
            }
            if last.elapsed() >= tick {
                last = Instant::now();
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;
    result
}

// layout

fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let core_count = app.cpu_info.as_ref().map(|c| c.core_count).unwrap_or(4);
    let core_rows = ((core_count + 1) / 2).clamp(2, 8) as u16;
    let cores_height = core_rows + 2;

    let chunks = Layout::vertical([
        Constraint::Length(3),            // header
        Constraint::Length(cores_height), // per-core panel
        Constraint::Length(6),            // cpu history + mem
        Constraint::Min(8),               // hot functions
        Constraint::Length(1),            // footer
    ])
    .split(area);

    draw_header(f, app, chunks[0]);
    draw_cores(f, app, chunks[1]);
    draw_timelines(f, app, chunks[2]);
    draw_table(f, app, chunks[3]);
    draw_footer(f, app, chunks[4]);
}

// header

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let pct = app.total_cpu_pct();
    let pct_col = heat(pct.min(100.0)); // heat based on single-core equivalent

    let status = if app.paused {
        Span::styled(" ⏸ PAUSED ", Style::default().fg(AMBER).bold())
    } else {
        Span::styled(" ● LIVE ", Style::default().fg(ACCENT).bold())
    };

    let line = Line::from(vec![
        Span::styled("  𒀭 nusku", Style::default().fg(ACCENT).bold()),
        Span::styled("  │  ", Style::default().fg(BORDER)),
        Span::styled(&app.cmd, Style::default().fg(CYAN)),
        Span::styled("  │  pid ", Style::default().fg(MUTED)),
        Span::styled(app.pid.to_string(), Style::default().fg(FG)),
        Span::styled("  │  cpu ", Style::default().fg(MUTED)),
        Span::styled(format!("{pct:.1}%"), Style::default().fg(pct_col).bold()),
        Span::styled("  │  mem ", Style::default().fg(MUTED)),
        Span::styled(
            app.snapshot
                .as_ref()
                .map(|s| fmt_kb(s.mem.rss_kb))
                .unwrap_or_default(),
            Style::default().fg(ACCENT),
        ),
        Span::styled("  │  up ", Style::default().fg(MUTED)),
        Span::styled(fmt_uptime(app.uptime), Style::default().fg(FG)),
        Span::styled("  │  samples ", Style::default().fg(MUTED)),
        Span::styled(fmt_count(app.total), Style::default().fg(FG)),
        Span::raw("  "),
        status,
    ]);

    f.render_widget(
        Paragraph::new(line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .border_style(Style::default().fg(BORDER))
                    .style(Style::default().bg(SURFACE)),
            )
            .alignment(Alignment::Left),
        area,
    );
}

// per-core CPU panel

fn draw_cores(f: &mut Frame, app: &App, area: Rect) {
    let pct = app.total_cpu_pct();
    let thread_count = app.cpu_info.as_ref().map(|c| c.threads.len()).unwrap_or(0);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" CPU CORES ", Style::default().fg(MUTED)),
            Span::styled(
                format!("  {pct:.1}% total  {thread_count} threads "),
                Style::default().fg(DIM),
            ),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let core_count = app.cpu_info.as_ref().map(|c| c.core_count).unwrap_or(4);

    // Build per-core percent array from /proc data
    let mut core_pcts = vec![0.0f64; core_count];
    if let Some(info) = &app.cpu_info {
        for cu in &info.active_cores {
            if cu.core_id < core_count {
                core_pcts[cu.core_id] = cu.percent;
            }
        }
    }

    let half = (core_count + 1) / 2;
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(inner);

    render_core_column(f, &core_pcts[..half.min(core_count)], 0, left);
    if half < core_count {
        render_core_column(f, &core_pcts[half..], half, right);
    }
}

fn render_core_column(f: &mut Frame, pcts: &[f64], start_id: usize, area: Rect) {
    let bar_w = (area.width as usize).saturating_sub(14).max(4);

    let lines: Vec<Line> = pcts
        .iter()
        .enumerate()
        .map(|(i, &pct)| {
            let id = start_id + i;
            let col = heat(pct);
            let filled = ((pct / 100.0) * bar_w as f64).round() as usize;
            let empty = bar_w.saturating_sub(filled);
            let active = pct > 0.5;

            Line::from(vec![
                Span::styled(
                    format!("C{id:<2} "),
                    Style::default().fg(if active { FG } else { DIM }),
                ),
                Span::styled("[", Style::default().fg(BORDER)),
                Span::styled("█".repeat(filled), Style::default().fg(col)),
                Span::styled("░".repeat(empty), Style::default().fg(DIM)),
                Span::styled("]", Style::default().fg(BORDER)),
                Span::styled(
                    format!(" {:5.1}%", pct),
                    Style::default().fg(if active { col } else { DIM }).bold(),
                ),
            ])
        })
        .collect();

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(SURFACE)),
        area,
    );
}

// cpu history + mem

fn draw_timelines(f: &mut Frame, app: &App, area: Rect) {
    let [cpu_panel, mem_panel] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(area);

    // CPU sparkline — data from /proc, correctly rises and falls
    let pct = app.total_cpu_pct();
    let col = heat(pct.min(100.0));
    // Sparkline max adapts to multi-core (could be 200%, 400%, etc.)
    let cpu_max = app
        .cpu_history
        .iter()
        .cloned()
        .max()
        .unwrap_or(100)
        .max(100);

    let cpu_block = Block::default()
        .title(Line::from(vec![
            Span::styled(" CPU HISTORY ", Style::default().fg(MUTED)),
            Span::styled(format!("{pct:.1}%"), Style::default().fg(col).bold()),
            Span::styled(
                if cpu_max > 100 {
                    format!("  (max {}%)", cpu_max)
                } else {
                    String::new()
                },
                Style::default().fg(DIM),
            ),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(SURFACE));

    let cpu_inner = cpu_block.inner(cpu_panel);
    f.render_widget(cpu_block, cpu_panel);

    let cpu_data: Vec<u64> = app.cpu_history.iter().cloned().collect();
    f.render_widget(
        Sparkline::default()
            .data(&cpu_data)
            .max(cpu_max)
            .style(Style::default().fg(col)),
        cpu_inner,
    );

    // Memory panel
    let (rss, virt) = app
        .snapshot
        .as_ref()
        .map(|s| (s.mem.rss_kb, s.mem.virt_kb))
        .unwrap_or((0, 0));
    let peak = app.mem_history.iter().cloned().max().unwrap_or(1).max(1);

    let mem_block = Block::default()
        .title(Line::from(vec![
            Span::styled(" MEM ", Style::default().fg(MUTED)),
            Span::styled(fmt_kb(rss), Style::default().fg(ACCENT).bold()),
            Span::styled("  virt ", Style::default().fg(DIM)),
            Span::styled(fmt_kb(virt), Style::default().fg(MUTED)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(SURFACE));

    let mem_inner = mem_block.inner(mem_panel);
    f.render_widget(mem_block, mem_panel);

    let [gauge_area, spark_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(mem_inner);

    f.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(ACCENT).bg(SURFACE))
            .ratio((rss as f64 / peak as f64).clamp(0.0, 1.0))
            .label(Span::styled(fmt_kb(rss), Style::default().fg(FG).bold())),
        gauge_area,
    );

    let mem_data: Vec<u64> = app.mem_history.iter().cloned().collect();
    f.render_widget(
        Sparkline::default()
            .data(&mem_data)
            .max(peak)
            .style(Style::default().fg(ACCENT)),
        spark_area,
    );
}

// hot functions table

fn draw_table(f: &mut Frame, app: &mut App, area: Rect) {
    let samples_label = app
        .snapshot
        .as_ref()
        .map(|s| format!("  {} samples/sec ", s.cpu.total_samples))
        .unwrap_or_else(|| "  waiting... ".to_string());

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" HOT FUNCTIONS ", Style::default().fg(MUTED)),
            Span::styled(samples_label, Style::default().fg(DIM)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG));

    let header = Row::new([
        Cell::from(Span::styled(
            "HEAT        %",
            Style::default().fg(MUTED).bold(),
        )),
        Cell::from(Span::styled("COUNT", Style::default().fg(MUTED).bold())),
        Cell::from(Span::styled("FUNCTION", Style::default().fg(MUTED).bold())),
        Cell::from(Span::styled("FILE", Style::default().fg(MUTED).bold())),
        Cell::from(Span::styled("LINE", Style::default().fg(MUTED).bold())),
    ])
    .style(Style::default().bg(HEADER_BG))
    .height(1);

    let rows: Vec<Row> = match &app.snapshot {
        None => vec![Row::new([
            Cell::from(""),
            Cell::from(""),
            Cell::from(Span::styled(
                "  waiting for samples...",
                Style::default().fg(DIM),
            )),
            Cell::from(""),
            Cell::from(""),
        ])],
        Some(snap) => snap
            .cpu
            .frames
            .iter()
            .map(|frame| {
                let col = heat(frame.percent);
                let filled = ((frame.percent / 100.0) * 8.0) as usize;
                let bar = format!(
                    "{}{} {:5.1}%",
                    "█".repeat(filled),
                    "░".repeat(8 - filled),
                    frame.percent,
                );
                Row::new([
                    Cell::from(Span::styled(bar, Style::default().fg(col).bold())),
                    Cell::from(Span::styled(
                        format!("{:>6}", frame.count),
                        Style::default().fg(MUTED),
                    )),
                    Cell::from(Span::styled(
                        truncate(&frame.name, 50),
                        Style::default().fg(FG),
                    )),
                    Cell::from(Span::styled(
                        frame.file.as_deref().unwrap_or("—").to_string(),
                        Style::default().fg(CYAN),
                    )),
                    Cell::from(Span::styled(
                        frame
                            .line
                            .map(|l| l.to_string())
                            .unwrap_or_else(|| "—".into()),
                        Style::default().fg(DIM),
                    )),
                ])
                .height(1)
            })
            .collect(),
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Length(7),
            Constraint::Min(26),
            Constraint::Length(18),
            Constraint::Length(5),
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(22, 30, 44))
            .fg(ACCENT)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ")
    .column_spacing(1);

    f.render_stateful_widget(table, area, &mut app.table_state);
}

// footer

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![
        Span::styled(" ↑↓/jk ", Style::default().fg(ACCENT)),
        Span::styled("navigate  ", Style::default().fg(DIM)),
        Span::styled("p ", Style::default().fg(ACCENT)),
        Span::styled("pause  ", Style::default().fg(DIM)),
        Span::styled("q ", Style::default().fg(ACCENT)),
        Span::styled("quit", Style::default().fg(DIM)),
    ];
    if app.paused {
        spans.push(Span::styled(
            "  ⏸ PAUSED — press p to resume",
            Style::default().fg(AMBER),
        ));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(SURFACE).fg(DIM)),
        area,
    );
}

fn fmt_uptime(s: u64) -> String {
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m{}s", s / 60, s % 60)
    } else {
        format!("{}h{}m", s / 3600, (s % 3600) / 60)
    }
}
fn fmt_count(n: u64) -> String {
    if n < 1_000 {
        n.to_string()
    } else if n < 1_000_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    }
}
fn fmt_kb(kb: u64) -> String {
    if kb < 1024 {
        format!("{kb}KB")
    } else if kb < 1024 * 1024 {
        format!("{:.1}MB", kb as f64 / 1024.0)
    } else {
        format!("{:.1}GB", kb as f64 / (1024.0 * 1024.0))
    }
}
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let end = s
        .char_indices()
        .nth(max - 1)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    format!("{}…", &s[..end])
}
