// scribe 0.2 — a small rescue TUI with a pluggable simulation hook.
//
// WHAT IT DOES
//   Scans a set of "important areas" under a root (default /mnt/sys), shows a
//   heatmap of where the data weight lives, and lets you select/deselect by
//   FOLDER (drill-down) or by EXTENSION. From that selection it produces a
//   machine-readable PLAN that any external tool can act on.
//
// THE HOOK (this is the part you build your mod against)
//   * Press 'w' to write three artifacts:
//       - scribe-plan.json      <- the contract: selection + file list + sizes
//       - scribe-selection.txt  <- plain rsync --files-from manifest
//       - scribe-copy.sh        <- ready-to-run rsync to your backup drive
//   * Run with `--executor /path/to/your-mod`. Then press 'x' to pipe the live
//     scribe-plan.json to your mod on STDIN; whatever it prints on STDOUT is
//     shown in a popup. That lets your mod SIMULATE the outcome with no teardown
//     of scribe — it just reads stdin, writes stdout.
//   * Run with `--json` to dump the scan as JSON and exit (no TUI), for piping.
//
// SAFETY
//   scribe only READS the scanned tree. It writes the three artifacts in the
//   current directory and nothing else. No file is copied or deleted until YOU
//   run scribe-copy.sh (or your mod does).
//
// KEYS
//   Tab  switch Folders<->Extensions   ↑/↓ (j/k) move   Space toggle select
//   a/n  all / none (current view)     w write plan     x simulate via executor
//   Esc  close popup                   q quit

use std::collections::{BTreeMap, HashSet};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap},
};
use walkdir::WalkDir;

const VERSION: &str = "0.2.0";
const DEFAULT_AREAS: &[&str] = &[
    "home", "medical_backups", "backup", "recovered", "forensics", "images",
    "srv", "opt", "GROK", "mnt2",
];

struct FileRec {
    rel: String,   // path relative to root, e.g. "home/khet/notes.txt"
    size: u64,
    ext: String,
    group: String, // folder group key at the configured depth, e.g. "home/khet"
}

#[derive(Clone, Copy, PartialEq)]
enum View {
    Folders,
    Exts,
}

struct Row {
    key: String,
    size: u64,
    count: u64,
}

struct Config {
    root: PathBuf,
    areas: Vec<String>,
    depth: usize,
    executor: Option<String>,
}

struct App {
    cfg: Config,
    files: Vec<FileRec>,
    total_bytes: u64,
    folders: Vec<Row>,
    exts: Vec<Row>,
    view: View,
    state: ListState,
    sel_folders: HashSet<String>,
    sel_exts: HashSet<String>,
    status: String,
    popup: Option<String>,
}

impl App {
    fn current_rows(&self) -> &[Row] {
        match self.view {
            View::Folders => &self.folders,
            View::Exts => &self.exts,
        }
    }

    fn is_selected(&self, key: &str) -> bool {
        match self.view {
            View::Folders => self.sel_folders.contains(key),
            View::Exts => self.sel_exts.contains(key),
        }
    }

    fn toggle_current(&mut self) {
        let idx = self.state.selected().unwrap_or(0);
        let key = match self.current_rows().get(idx) {
            Some(r) => r.key.clone(),
            None => return,
        };
        let set = match self.view {
            View::Folders => &mut self.sel_folders,
            View::Exts => &mut self.sel_exts,
        };
        if !set.remove(&key) {
            set.insert(key);
        }
    }

    fn select_all(&mut self, all: bool) {
        let keys: Vec<String> = self.current_rows().iter().map(|r| r.key.clone()).collect();
        let set = match self.view {
            View::Folders => &mut self.sel_folders,
            View::Exts => &mut self.sel_exts,
        };
        set.clear();
        if all {
            set.extend(keys);
        }
    }

    fn selected_files(&self) -> Vec<&FileRec> {
        self.files
            .iter()
            .filter(|f| self.sel_folders.contains(&f.group) || self.sel_exts.contains(&f.ext))
            .collect()
    }

    fn selected_summary(&self) -> (u64, u64) {
        let v = self.selected_files();
        (v.iter().map(|f| f.size).sum(), v.len() as u64)
    }

    fn move_by(&mut self, delta: isize) {
        let len = self.current_rows().len();
        if len == 0 {
            return;
        }
        let cur = self.state.selected().unwrap_or(0) as isize;
        let next = (cur + delta).clamp(0, len as isize - 1);
        self.state.select(Some(next as usize));
    }

    /// The hook contract. Stable, machine-readable. Build your mod against this.
    fn build_plan_json(&self) -> String {
        let files = self.selected_files();
        let (bytes, n) = (files.iter().map(|f| f.size).sum::<u64>(), files.len());
        let mut folders: Vec<&String> = self.sel_folders.iter().collect();
        let mut exts: Vec<&String> = self.sel_exts.iter().collect();
        folders.sort();
        exts.sort();

        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"scribe_version\": \"{}\",\n", VERSION));
        s.push_str(&format!("  \"root\": \"{}\",\n", json_esc(&self.cfg.root.to_string_lossy())));
        s.push_str("  \"selection\": {\n");
        s.push_str(&format!("    \"folders\": [{}],\n", json_arr(&folders)));
        s.push_str(&format!("    \"extensions\": [{}]\n", json_arr(&exts)));
        s.push_str("  },\n");
        s.push_str(&format!("  \"summary\": {{ \"files\": {}, \"bytes\": {} }},\n", n, bytes));
        s.push_str("  \"files\": [\n");
        for (i, f) in files.iter().enumerate() {
            let comma = if i + 1 < files.len() { "," } else { "" };
            s.push_str(&format!(
                "    {{ \"path\": \"{}\", \"bytes\": {} }}{}\n",
                json_esc(&f.rel),
                f.size,
                comma
            ));
        }
        s.push_str("  ]\n}\n");
        s
    }

    fn write_plan(&mut self) {
        let files = self.selected_files();
        if files.is_empty() {
            self.status = "Nothing selected — pick folders/extensions first (Space).".into();
            return;
        }
        let lines: Vec<String> = files.iter().map(|f| f.rel.clone()).collect();

        let plan = self.build_plan_json();
        let copy = format!(
            "#!/usr/bin/env bash\n# Generated by scribe {v}. Edit DEST then run.\nset -e\nDEST=\"/mnt/backup/KHETPRIME-rescue\"\nmkdir -p \"$DEST\"\nrsync -aAX --info=progress2 --files-from=scribe-selection.txt \"{root}\" \"$DEST/\"\necho \"Done: copied {n} files to $DEST\"\n",
            v = VERSION,
            root = self.cfg.root.display(),
            n = lines.len(),
        );

        let r1 = std::fs::write("scribe-selection.txt", lines.join("\n") + "\n");
        let r2 = std::fs::write("scribe-plan.json", &plan);
        let r3 = std::fs::write("scribe-copy.sh", copy);
        self.status = match (r1, r2, r3) {
            (Ok(_), Ok(_), Ok(_)) => format!(
                "Wrote scribe-plan.json / -selection.txt / -copy.sh  ({} files). Run: bash scribe-copy.sh",
                lines.len()
            ),
            _ => "ERROR writing artifacts (permission? disk full?)".into(),
        };
    }

    fn simulate(&mut self) {
        let prog = match &self.cfg.executor {
            Some(p) => p.clone(),
            None => {
                self.status = "No --executor set. Run scribe with --executor /path/to/your-mod.".into();
                return;
            }
        };
        if self.selected_files().is_empty() {
            self.status = "Nothing selected to simulate.".into();
            return;
        }
        let plan = self.build_plan_json();
        self.popup = Some(run_executor(&prog, &plan));
    }
}

fn json_esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

fn json_arr(items: &[&String]) -> String {
    items
        .iter()
        .map(|s| format!("\"{}\"", json_esc(s)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn run_executor(prog: &str, plan: &str) -> String {
    let mut parts = prog.split_whitespace();
    let exe = match parts.next() {
        Some(e) => e,
        None => return "empty executor".into(),
    };
    let args: Vec<&str> = parts.collect();
    let child = Command::new(exe)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    match child {
        Ok(mut c) => {
            if let Some(mut si) = c.stdin.take() {
                let _ = si.write_all(plan.as_bytes());
            }
            match c.wait_with_output() {
                Ok(out) => {
                    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
                    if !out.stderr.is_empty() {
                        s.push_str("\n--- stderr ---\n");
                        s.push_str(&String::from_utf8_lossy(&out.stderr));
                    }
                    if s.trim().is_empty() {
                        s = "(executor produced no output)".into();
                    }
                    s
                }
                Err(e) => format!("executor wait failed: {e}"),
            }
        }
        Err(e) => format!("could not start executor '{prog}': {e}"),
    }
}

fn ext_of(p: &Path) -> String {
    match p.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_lowercase(),
        None => "(no ext)".into(),
    }
}

fn group_of(rel: &str, depth: usize) -> String {
    let mut comps: Vec<&str> = rel.split('/').collect();
    comps.pop(); // drop filename
    if comps.is_empty() {
        return "(root files)".into();
    }
    let take = depth.min(comps.len()).max(1);
    comps[..take].join("/")
}

fn scan(cfg: &Config) -> Vec<FileRec> {
    let mut files = Vec::new();
    for area in &cfg.areas {
        let base = cfg.root.join(area);
        if !base.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&base).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let rel = match entry.path().strip_prefix(&cfg.root) {
                Ok(r) => r.to_string_lossy().into_owned(),
                Err(_) => continue,
            };
            let group = group_of(&rel, cfg.depth);
            files.push(FileRec {
                ext: ext_of(entry.path()),
                rel,
                size,
                group,
            });
        }
    }
    files
}

fn aggregate(files: &[FileRec], by_folder: bool) -> Vec<Row> {
    let mut map: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for f in files {
        let key = if by_folder { &f.group } else { &f.ext };
        let e = map.entry(key.clone()).or_insert((0, 0));
        e.0 += f.size;
        e.1 += 1;
    }
    let mut rows: Vec<Row> = map
        .into_iter()
        .map(|(key, (size, count))| Row { key, size, count })
        .collect();
    rows.sort_by(|a, b| b.size.cmp(&a.size));
    rows
}

fn human(bytes: u64) -> String {
    const U: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", bytes, U[0])
    } else {
        format!("{:.1} {}", v, U[i])
    }
}

fn heat_bar(size: u64, max: u64, width: usize) -> (String, Color) {
    let ratio = if max == 0 { 0.0 } else { size as f64 / max as f64 };
    let mut filled = (ratio * width as f64).round() as usize;
    if size > 0 && filled == 0 {
        filled = 1;
    }
    let bar = "█".repeat(filled);
    let pad = " ".repeat(width.saturating_sub(filled));
    let color = if ratio >= 0.75 {
        Color::Red
    } else if ratio >= 0.50 {
        Color::LightRed
    } else if ratio >= 0.30 {
        Color::Yellow
    } else if ratio >= 0.15 {
        Color::Green
    } else if ratio > 0.0 {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    (format!("{bar}{pad}"), color)
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

fn tab(label: &str, active: bool) -> Span<'static> {
    if active {
        Span::styled(
            format!(" {label} "),
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(format!(" {label} "), Style::default().fg(Color::Gray))
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(6)])
        .split(f.area());

    // ---- header / tab bar ----
    let header = Paragraph::new(Line::from(vec![
        Span::styled(format!(" scribe {VERSION} "), Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw("  "),
        tab("Folders", app.view == View::Folders),
        Span::raw(" "),
        tab("Extensions", app.view == View::Exts),
        Span::raw(format!("   root: {}", app.cfg.root.display())),
        Span::raw(match &app.cfg.executor {
            Some(e) => format!("   executor: {e}"),
            None => "   executor: (none)".into(),
        }),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // ---- main list ----
    let rows = app.current_rows();
    let max = rows.first().map(|r| r.size).unwrap_or(0);
    let items: Vec<ListItem> = rows
        .iter()
        .map(|r| {
            let checked = app.is_selected(&r.key);
            let mark = if checked { "[x] " } else { "[ ] " };
            let (bar, color) = heat_bar(r.size, max, 22);
            ListItem::new(Line::from(vec![
                Span::styled(mark, Style::default().fg(if checked { Color::Green } else { Color::DarkGray })),
                Span::raw(format!("{:<28}", truncate(&r.key, 28))),
                Span::raw(format!("{:>9}", human(r.size))),
                Span::raw(format!("{:>8}  ", r.count)),
                Span::styled(bar, Style::default().fg(color)),
            ]))
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " {} items — Space pick · a all · n none ",
            rows.len()
        )))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_stateful_widget(list, chunks[1], &mut app.state);

    // ---- footer: gauge + summary + status ----
    let foot = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(2)])
        .split(chunks[2].inner(Margin { horizontal: 1, vertical: 1 }));
    f.render_widget(Block::default().borders(Borders::ALL), chunks[2]);

    let (sel_bytes, sel_n) = app.selected_summary();
    let pct = if app.total_bytes == 0 {
        0.0
    } else {
        sel_bytes as f64 / app.total_bytes as f64
    };
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(pct.clamp(0.0, 1.0))
        .label(format!(
            "{} / {} selected ({:.0}%)",
            human(sel_bytes),
            human(app.total_bytes),
            pct * 100.0
        ));
    f.render_widget(gauge, foot[0]);

    let summary = Paragraph::new(Line::from(vec![
        Span::styled("Selected: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!("{sel_n} files, {}", human(sel_bytes)), Style::default().fg(Color::Green)),
        Span::raw(format!("   folders:{}  exts:{}", app.sel_folders.len(), app.sel_exts.len())),
    ]));
    f.render_widget(summary, foot[1]);

    let help = "Tab switch · ↑↓ move · Space pick · a/n all/none · w write plan · x simulate · q quit";
    let status = Paragraph::new(if app.status.is_empty() { help.to_string() } else { app.status.clone() })
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    f.render_widget(status, foot[2]);

    // ---- popup (simulation output) ----
    if let Some(text) = &app.popup {
        let area = centered(70, 70, f.area());
        f.render_widget(Clear, area);
        let p = Paragraph::new(text.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" simulation output — Esc to close ")
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
    }
}

fn centered(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(v[1])[1]
}

fn run(terminal: &mut Terminal<impl Backend>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(k) = event::read()? {
            if k.kind != KeyEventKind::Press {
                continue;
            }
            // Popup eats most keys until closed.
            if app.popup.is_some() {
                match k.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('x') => app.popup = None,
                    KeyCode::Char('q') => return Ok(()),
                    _ => {}
                }
                continue;
            }
            app.status.clear();
            match k.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Tab => {
                    app.view = match app.view {
                        View::Folders => View::Exts,
                        View::Exts => View::Folders,
                    };
                    app.state.select(Some(0));
                }
                KeyCode::Down | KeyCode::Char('j') => app.move_by(1),
                KeyCode::Up | KeyCode::Char('k') => app.move_by(-1),
                KeyCode::Char(' ') | KeyCode::Enter => app.toggle_current(),
                KeyCode::Char('a') => app.select_all(true),
                KeyCode::Char('n') => app.select_all(false),
                KeyCode::Char('w') => app.write_plan(),
                KeyCode::Char('x') => app.simulate(),
                _ => {}
            }
        }
    }
}

fn parse_args() -> (Config, bool) {
    let mut root: Option<PathBuf> = None;
    let mut areas: Option<Vec<String>> = None;
    let mut depth = 2usize;
    let mut executor: Option<String> = None;
    let mut json = false;

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--json" => json = true,
            "--executor" => executor = it.next(),
            "--depth" => depth = it.next().and_then(|s| s.parse().ok()).unwrap_or(2),
            "--areas" => {
                areas = it.next().map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other if !other.starts_with('-') => root = Some(PathBuf::from(other)),
            other => {
                eprintln!("scribe: unknown flag '{other}' (try --help)");
                std::process::exit(2);
            }
        }
    }

    let cfg = Config {
        root: root.unwrap_or_else(|| PathBuf::from("/mnt/sys")),
        areas: areas.unwrap_or_else(|| DEFAULT_AREAS.iter().map(|s| s.to_string()).collect()),
        depth: depth.max(1),
        executor,
    };
    (cfg, json)
}

fn print_help() {
    println!(
        "scribe {VERSION} — scan, heatmap, select-to-backup, with a simulation hook\n\n\
         USAGE:\n  scribe [ROOT] [--areas a,b,c] [--depth N] [--executor PROG] [--json]\n\n\
         ROOT          directory to scan under (default /mnt/sys)\n\
         --areas       comma list of top folders to scan (default: {areas})\n\
         --depth       folder-grouping depth for the Folders view (default 2)\n\
         --executor    program to pipe scribe-plan.json to on 'x' (your sim mod)\n\
         --json        print the scan as JSON and exit (no TUI)\n\n\
         In the TUI press 'w' to write scribe-plan.json / -selection.txt / -copy.sh.",
        areas = DEFAULT_AREAS.join(",")
    );
}

fn main() -> io::Result<()> {
    let (cfg, json_mode) = parse_args();

    if !cfg.root.is_dir() {
        eprintln!("scribe: '{}' is not a directory. (try --help)", cfg.root.display());
        std::process::exit(1);
    }

    eprintln!("scribe: scanning {} ...", cfg.root.display());
    let files = scan(&cfg);
    if files.is_empty() {
        eprintln!(
            "scribe: no files in the scanned areas under {}.\n  areas: {}",
            cfg.root.display(),
            cfg.areas.join(", ")
        );
        std::process::exit(1);
    }
    let total_bytes = files.iter().map(|f| f.size).sum();
    let folders = aggregate(&files, true);
    let exts = aggregate(&files, false);

    if json_mode {
        // Non-interactive: emit the full scan (everything, nothing selected).
        let mut s = String::from("{\n  \"scribe_version\": \"");
        s.push_str(VERSION);
        s.push_str("\",\n");
        s.push_str(&format!("  \"root\": \"{}\",\n", json_esc(&cfg.root.to_string_lossy())));
        s.push_str(&format!("  \"total_bytes\": {total_bytes},\n"));
        s.push_str("  \"folders\": [\n");
        for (i, r) in folders.iter().enumerate() {
            let c = if i + 1 < folders.len() { "," } else { "" };
            s.push_str(&format!(
                "    {{ \"key\": \"{}\", \"bytes\": {}, \"files\": {} }}{}\n",
                json_esc(&r.key), r.size, r.count, c
            ));
        }
        s.push_str("  ],\n  \"extensions\": [\n");
        for (i, r) in exts.iter().enumerate() {
            let c = if i + 1 < exts.len() { "," } else { "" };
            s.push_str(&format!(
                "    {{ \"key\": \"{}\", \"bytes\": {}, \"files\": {} }}{}\n",
                json_esc(&r.key), r.size, r.count, c
            ));
        }
        s.push_str("  ]\n}\n");
        print!("{s}");
        return Ok(());
    }

    let mut state = ListState::default();
    state.select(Some(0));

    let mut app = App {
        cfg,
        files,
        total_bytes,
        folders,
        exts,
        view: View::Folders,
        state,
        sel_folders: HashSet::new(),
        sel_exts: HashSet::new(),
        status: String::new(),
        popup: None,
    };

    let mut terminal = ratatui::init();
    let res = run(&mut terminal, &mut app);
    ratatui::restore();
    res
}
