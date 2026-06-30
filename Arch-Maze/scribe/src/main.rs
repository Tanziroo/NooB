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
//   a/n  all / none (visible rows)     /  filter rows   w write (asks to confirm)
//   x    simulate via executor         Esc clear filter / close popup   q quit
//
// LAYERED ARCHITECTURE (see LAYERS.md). One anchor feature per layer:
//   L0 manifest  — fingerprint() over the selection, embedded in every artifact
//   L1 approval  — `w` opens a confirm popup; commit_write only runs on `y`
//   L2 journal   — scribe-journal.json: resumable per-file backup state
//   L3 scale     — live `/` filter narrows the list; a/n act on visible rows
//   L4 audit     — scribe-session.log: append-only record of every commit

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

const VERSION: &str = "0.4.0";
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

struct SimRow {
    key: String,
    bytes: u64,
    action: String,
}

/// Structured result an executor may return on stdout (SPEC §7.2).
struct SimResult {
    dest: String,
    dest_free: u64,
    fits: bool,
    eta_seconds: u64,
    copy_bytes: u64,
    skip_bytes: u64,
    conflicts: u64,
    rows: Vec<SimRow>,
}

enum Popup {
    Text(String),
    Sim(SimResult),
    Confirm(String), // L1: approval gate — shown before any artifacts are written
}

/// Parse an executor's stdout as a sim-result. Returns None if it isn't valid
/// sim JSON, so the caller can fall back to showing the raw text.
fn parse_sim(out: &str) -> Option<SimResult> {
    let v: serde_json::Value = serde_json::from_str(out.trim()).ok()?;
    // Require the v0.3 shape: must have a "rows" array to count as a sim-result.
    let rows_v = v.get("rows")?.as_array()?;
    let u64f = |val: &serde_json::Value, k: &str| -> u64 {
        val.get(k).and_then(|x| x.as_u64()).unwrap_or(0)
    };
    let totals = v.get("totals").cloned().unwrap_or(serde_json::Value::Null);
    let rows = rows_v
        .iter()
        .map(|r| SimRow {
            key: r.get("key").and_then(|x| x.as_str()).unwrap_or("?").to_string(),
            bytes: u64f(r, "bytes"),
            action: r
                .get("action")
                .and_then(|x| x.as_str())
                .unwrap_or("COPY")
                .to_uppercase(),
        })
        .collect();
    Some(SimResult {
        dest: v.get("dest").and_then(|x| x.as_str()).unwrap_or("(dest?)").to_string(),
        dest_free: u64f(&v, "dest_free_bytes"),
        fits: v.get("fits").and_then(|x| x.as_bool()).unwrap_or(true),
        eta_seconds: u64f(&v, "eta_seconds"),
        copy_bytes: u64f(&totals, "copy_bytes"),
        skip_bytes: u64f(&totals, "skip_bytes"),
        conflicts: u64f(&totals, "conflicts"),
        rows,
    })
}

fn action_color(action: &str) -> Color {
    match action {
        "COPY" => Color::Green,
        "SKIP" => Color::Yellow,
        "CONFLICT" => Color::Red,
        "DEDUP" => Color::Cyan,
        _ => Color::Gray,
    }
}

fn fmt_eta(secs: u64) -> String {
    if secs == 0 {
        "—".into()
    } else if secs < 90 {
        format!("~{secs}s")
    } else if secs < 5400 {
        format!("~{}m", (secs + 30) / 60)
    } else {
        format!("~{:.1}h", secs as f64 / 3600.0)
    }
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
    popup: Option<Popup>,
    filter: String,    // L3: live filter text (narrows the visible list)
    filter_mode: bool, // L3: true while the operator is typing the filter
}

impl App {
    /// All rows for the current view, narrowed by the live filter (L3).
    /// Every navigation/selection action operates over THIS set, so filtering
    /// and selecting compose: filter to "med", press `a`, and only the matching
    /// rows are picked.
    fn visible_rows(&self) -> Vec<&Row> {
        let all = match self.view {
            View::Folders => &self.folders,
            View::Exts => &self.exts,
        };
        if self.filter.is_empty() {
            all.iter().collect()
        } else {
            let needle = self.filter.to_lowercase();
            all.iter().filter(|r| r.key.to_lowercase().contains(&needle)).collect()
        }
    }

    fn reset_cursor(&mut self) {
        self.state.select(Some(0));
    }

    fn is_selected(&self, key: &str) -> bool {
        match self.view {
            View::Folders => self.sel_folders.contains(key),
            View::Exts => self.sel_exts.contains(key),
        }
    }

    fn toggle_current(&mut self) {
        let idx = self.state.selected().unwrap_or(0);
        let key = {
            let vis = self.visible_rows();
            match vis.get(idx) {
                Some(r) => r.key.clone(),
                None => return,
            }
        };
        let set = match self.view {
            View::Folders => &mut self.sel_folders,
            View::Exts => &mut self.sel_exts,
        };
        if !set.remove(&key) {
            set.insert(key);
        }
    }

    /// `a`/`n` act on the VISIBLE rows. With no filter this is "all/none"; with a
    /// filter it adds/removes just the matching rows, preserving other picks.
    fn select_all(&mut self, all: bool) {
        let keys: Vec<String> = self.visible_rows().iter().map(|r| r.key.clone()).collect();
        let filtered = !self.filter.is_empty();
        let set = match self.view {
            View::Folders => &mut self.sel_folders,
            View::Exts => &mut self.sel_exts,
        };
        if filtered {
            if all {
                set.extend(keys);
            } else {
                for k in &keys {
                    set.remove(k);
                }
            }
        } else {
            set.clear();
            if all {
                set.extend(keys);
            }
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
        let len = self.visible_rows().len();
        if len == 0 {
            self.state.select(Some(0));
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
        // L0: deterministic fingerprint of the exact selection. Downstream tools
        // (and the journal) use it to confirm they're acting on the same set.
        s.push_str(&format!("  \"manifest_fingerprint\": \"{}\",\n", fingerprint(&files)));
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

    /// L1 (approval gate): `w` no longer writes immediately. It builds a summary
    /// — including the L0 fingerprint — and asks the operator to confirm. Nothing
    /// touches the filesystem until `commit_write`.
    fn prepare_write(&mut self) {
        let files = self.selected_files();
        if files.is_empty() {
            self.status = "Nothing selected — pick folders/extensions first (Space).".into();
            return;
        }
        let bytes: u64 = files.iter().map(|f| f.size).sum();
        let n = files.len();
        let fp = fingerprint(&files);
        let summary = format!(
            "About to write a backup plan:\n\n  \
             files:        {n}\n  \
             bytes:        {}\n  \
             fingerprint:  {fp}\n  \
             folders:      {}    extensions: {}\n\n\
             Writes (plans only — nothing is copied):\n  \
             • scribe-plan.json      (the contract + fingerprint)\n  \
             • scribe-selection.txt  (rsync manifest)\n  \
             • scribe-copy.sh        (ready-to-run rsync)\n  \
             • scribe-journal.json   (resumable backup state)\n  \
             • scribe-session.log    (append-only audit entry)\n\n\
             Commit?   [y] yes    [n] no",
            human(bytes),
            self.sel_folders.len(),
            self.sel_exts.len(),
        );
        self.popup = Some(Popup::Confirm(summary));
    }

    /// L2 + L4: the only place that writes to disk. Emits the plan, manifest,
    /// rsync script, a resumable journal (L2), and appends an immutable audit
    /// line to the session log (L4) — all keyed to one fingerprint (L0).
    fn commit_write(&mut self) {
        let files = self.selected_files();
        if files.is_empty() {
            self.status = "Nothing selected.".into();
            return;
        }
        let lines: Vec<String> = files.iter().map(|f| f.rel.clone()).collect();
        let bytes: u64 = files.iter().map(|f| f.size).sum();
        let fp = fingerprint(&files);
        let ts = now_epoch();

        let plan = self.build_plan_json();
        let copy = format!(
            "#!/usr/bin/env bash\n# Generated by scribe {v}. Edit DEST then run.\nset -e\nDEST=\"/mnt/backup/KHETPRIME-rescue\"\nmkdir -p \"$DEST\"\nrsync -aAX --info=progress2 --files-from=scribe-selection.txt \"{root}\" \"$DEST/\"\necho \"Done: copied {n} files to $DEST\"\n",
            v = VERSION,
            root = self.cfg.root.display(),
            n = lines.len(),
        );
        let journal = build_journal(&files, &fp, ts, &self.cfg.root);

        let r1 = std::fs::write("scribe-selection.txt", lines.join("\n") + "\n");
        let r2 = std::fs::write("scribe-plan.json", &plan);
        let r3 = std::fs::write("scribe-copy.sh", copy);
        let r4 = std::fs::write("scribe-journal.json", journal);
        let r5 = append_session_log(&format!(
            "ts={ts} version={VERSION} root={} files={} bytes={bytes} fp={fp} folders={} exts={}",
            self.cfg.root.display(),
            lines.len(),
            self.sel_folders.len(),
            self.sel_exts.len(),
        ));

        self.status = if r1.is_ok() && r2.is_ok() && r3.is_ok() && r4.is_ok() && r5.is_ok() {
            format!(
                "Committed {} files (fp {fp}): plan+manifest+journal written, session logged. Run: bash scribe-copy.sh",
                lines.len()
            )
        } else {
            "ERROR writing artifacts (permission? disk full?)".into()
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
        let out = run_executor(&prog, &plan);
        // Render a structured sim if the mod returned one; else show raw text.
        self.popup = Some(match parse_sim(&out) {
            Some(sim) => Popup::Sim(sim),
            None => Popup::Text(out),
        });
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

/// L0 anchor: a deterministic 64-bit FNV-1a fingerprint over the selection
/// (sorted `(path, size)` pairs). Order-independent and stable, so the same set
/// always yields the same hex string — the integrity key for plan + journal.
fn fingerprint(files: &[&FileRec]) -> String {
    let mut keyed: Vec<(&str, u64)> = files.iter().map(|f| (f.rel.as_str(), f.size)).collect();
    keyed.sort();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    for (rel, size) in keyed {
        for b in rel.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(PRIME);
        }
        h ^= size;
        h = h.wrapping_mul(PRIME);
    }
    format!("{h:016x}")
}

fn now_epoch() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// L2 anchor: the resumable backup journal. Every selected file starts `pending`;
/// a future `--resume` reads this, checks what already landed, and continues.
fn build_journal(files: &[&FileRec], fp: &str, ts: u64, root: &Path) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"scribe_version\": \"{VERSION}\",\n"));
    s.push_str(&format!("  \"created_epoch\": {ts},\n"));
    s.push_str(&format!("  \"root\": \"{}\",\n", json_esc(&root.to_string_lossy())));
    s.push_str(&format!("  \"manifest_fingerprint\": \"{fp}\",\n"));
    s.push_str("  \"status\": \"pending\",\n");
    s.push_str("  \"entries\": [\n");
    for (i, f) in files.iter().enumerate() {
        let c = if i + 1 < files.len() { "," } else { "" };
        s.push_str(&format!(
            "    {{ \"path\": \"{}\", \"bytes\": {}, \"status\": \"pending\" }}{}\n",
            json_esc(&f.rel),
            f.size,
            c
        ));
    }
    s.push_str("  ]\n}\n");
    s
}

/// L4 anchor: append-only audit. One line per committed plan, never rewritten.
fn append_session_log(line: &str) -> io::Result<()> {
    use std::fs::OpenOptions;
    let mut f = OpenOptions::new().create(true).append(true).open("scribe-session.log")?;
    writeln!(f, "{line}")
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
    let mut progress_bytes = 0u64;
    let mut progress_files = 0u64;
    let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut spin = 0usize;

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
            progress_bytes += size;
            progress_files += 1;

            // Report progress every 500 files.
            if progress_files % 500 == 0 {
                eprint!(
                    "\r{} scribe: {} files, {}   ",
                    spinner[spin % spinner.len()],
                    progress_files,
                    human(progress_bytes)
                );
                spin = spin.wrapping_add(1);
                let _ = io::stderr().flush();
            }
        }
    }
    if progress_files > 0 {
        eprintln!("\r✓ scribe: {} files, {}        ", progress_files, human(progress_bytes));
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

    // ---- main list (filtered, L3) ----
    let rows = app.visible_rows();
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
    let title = if app.filter.is_empty() {
        format!(" {} items — Space pick · a all · n none · / filter ", rows.len())
    } else {
        format!(
            " {} items — filter: \"{}{}\"  (Esc clear) ",
            rows.len(),
            app.filter,
            if app.filter_mode { "▏" } else { "" }
        )
    };
    let title_style = if app.filter_mode {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(title, title_style)),
        )
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

    let help = if app.filter_mode {
        "type to filter · Enter keep · Esc clear".to_string()
    } else {
        "Tab switch · ↑↓ move · Space pick · a/n all/none · / filter · w write · x simulate · q quit".to_string()
    };
    let status = Paragraph::new(if app.status.is_empty() { help } else { app.status.clone() })
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    f.render_widget(status, foot[2]);

    // ---- popup ----
    if let Some(popup) = &app.popup {
        let area = centered(72, 74, f.area());
        f.render_widget(Clear, area);
        match popup {
            Popup::Text(text) => {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(" simulation output — Esc to close ")
                    .border_style(Style::default().fg(Color::Magenta));
                f.render_widget(Paragraph::new(text.clone()).block(block).wrap(Wrap { trim: false }), area);
            }
            Popup::Sim(sim) => {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(" simulation output — Esc to close ")
                    .border_style(Style::default().fg(Color::Magenta));
                render_sim(f, area, block, sim);
            }
            Popup::Confirm(msg) => {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(" confirm write — [y] commit   [n] cancel ")
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
                f.render_widget(Paragraph::new(msg.clone()).block(block).wrap(Wrap { trim: false }), area);
            }
        }
    }
}

/// Render a structured sim-result: header, per-row projected bars + action
/// badges, and a "will it fit?" gauge (SPEC §7.2).
fn render_sim(f: &mut Frame, area: Rect, block: Block, sim: &SimResult) {
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("dest: ", Style::default().fg(Color::Gray)),
        Span::raw(sim.dest.clone()),
        Span::styled(format!("  ({} free)", human(sim.dest_free)), Style::default().fg(Color::Gray)),
    ]));
    let conflict_style = if sim.conflicts > 0 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    lines.push(Line::from(vec![
        Span::styled(format!("copy {}", human(sim.copy_bytes)), Style::default().fg(Color::Green)),
        Span::raw(" · "),
        Span::styled(format!("skip {}", human(sim.skip_bytes)), Style::default().fg(Color::Yellow)),
        Span::raw(" · "),
        Span::styled(format!("{} conflicts", sim.conflicts), conflict_style),
        Span::raw(" · "),
        Span::styled(format!("ETA {}", fmt_eta(sim.eta_seconds)), Style::default().fg(Color::Gray)),
    ]));
    lines.push(Line::from(""));

    let max = sim.rows.iter().map(|r| r.bytes).max().unwrap_or(0);
    for r in &sim.rows {
        let (bar, color) = heat_bar(r.bytes, max, 16);
        lines.push(Line::from(vec![
            Span::raw(format!("{:<18}", truncate(&r.key, 18))),
            Span::raw(format!("{:>8}  ", human(r.bytes))),
            Span::styled(bar, Style::default().fg(color)),
            Span::raw("  "),
            Span::styled(
                format!(" {} ", r.action),
                Style::default().fg(Color::Black).bg(action_color(&r.action)).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("will it fit?", Style::default().fg(Color::Gray))));

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(lines.len() as u16), Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), layout[0]);

    // Fit gauge: how full the destination is after the copy lands.
    let after = sim.copy_bytes;
    let denom = sim.dest_free.max(1);
    let ratio = (after as f64 / denom as f64).clamp(0.0, 1.0);
    let gcolor = if sim.fits { Color::Green } else { Color::Red };
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(gcolor))
        .ratio(ratio)
        .label(format!("{} of {} dest ({:.0}%)", human(after), human(sim.dest_free), ratio * 100.0));
    f.render_widget(gauge, layout[1]);

    let verdict = if sim.fits {
        Span::styled(format!("✓ fits · {} conflicts · ETA {}", sim.conflicts, fmt_eta(sim.eta_seconds)),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("✗ WILL NOT FIT — deselect something",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
    };
    f.render_widget(Paragraph::new(Line::from(verdict)), layout[2]);
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
            // Popup eats keys until closed. Confirm (L1) needs y/n; others close.
            if app.popup.is_some() {
                let is_confirm = matches!(app.popup, Some(Popup::Confirm(_)));
                if is_confirm {
                    match k.code {
                        KeyCode::Char('y') => {
                            app.popup = None;
                            app.commit_write();
                        }
                        KeyCode::Char('n') | KeyCode::Esc => {
                            app.popup = None;
                            app.status = "Write cancelled.".into();
                        }
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    }
                } else {
                    match k.code {
                        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('x') => app.popup = None,
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    }
                }
                continue;
            }

            // Filter typing (L3) intercepts keys before normal commands.
            if app.filter_mode {
                match k.code {
                    KeyCode::Esc => {
                        app.filter_mode = false;
                        app.filter.clear();
                        app.reset_cursor();
                    }
                    KeyCode::Enter => app.filter_mode = false,
                    KeyCode::Backspace => {
                        app.filter.pop();
                        app.reset_cursor();
                    }
                    KeyCode::Char(c) => {
                        app.filter.push(c);
                        app.reset_cursor();
                    }
                    _ => {}
                }
                continue;
            }

            app.status.clear();
            match k.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Esc => {
                    // Esc clears an active filter first, then quits.
                    if app.filter.is_empty() {
                        return Ok(());
                    }
                    app.filter.clear();
                    app.reset_cursor();
                }
                KeyCode::Char('/') => app.filter_mode = true,
                KeyCode::Tab => {
                    app.view = match app.view {
                        View::Folders => View::Exts,
                        View::Exts => View::Folders,
                    };
                    app.reset_cursor();
                }
                KeyCode::Down | KeyCode::Char('j') => app.move_by(1),
                KeyCode::Up | KeyCode::Char('k') => app.move_by(-1),
                KeyCode::Char(' ') | KeyCode::Enter => app.toggle_current(),
                KeyCode::Char('a') => app.select_all(true),
                KeyCode::Char('n') => app.select_all(false),
                KeyCode::Char('w') => app.prepare_write(),
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
         In the TUI: '/' filters the list, 'w' asks to confirm then writes\n\
         scribe-plan.json / -selection.txt / -copy.sh / -journal.json and\n\
         appends an audit line to scribe-session.log.",
        areas = DEFAULT_AREAS.join(",")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_sim_result() {
        let j = r#"{
            "dest":"/mnt/backup","dest_free_bytes":1000,"fits":true,"eta_seconds":420,
            "totals":{"copy_bytes":800,"skip_bytes":50,"conflicts":0},
            "rows":[{"key":"medical_backups","bytes":800,"action":"copy"},
                    {"key":"(dedup)","bytes":50,"action":"SKIP"}]
        }"#;
        let s = parse_sim(j).expect("should parse");
        assert_eq!(s.dest, "/mnt/backup");
        assert_eq!(s.copy_bytes, 800);
        assert!(s.fits);
        assert_eq!(s.rows.len(), 2);
        assert_eq!(s.rows[0].action, "COPY"); // upper-cased
    }

    #[test]
    fn rejects_non_sim_text() {
        assert!(parse_sim("just a log line, not json").is_none());
        assert!(parse_sim(r#"{"hello":"world"}"#).is_none()); // no rows[]
    }

    #[test]
    fn eta_formats() {
        assert_eq!(fmt_eta(0), "—");
        assert_eq!(fmt_eta(45), "~45s");
        assert_eq!(fmt_eta(420), "~7m");
    }

    #[test]
    fn group_depth() {
        assert_eq!(group_of("home/khet/Museum/x.pdf", 2), "home/khet");
        assert_eq!(group_of("images/a.png", 2), "images");
    }

    fn rec(rel: &str, size: u64) -> FileRec {
        FileRec { rel: rel.into(), size, ext: "x".into(), group: "g".into() }
    }

    #[test]
    fn fingerprint_is_stable_and_order_independent() {
        let a = rec("home/a.txt", 10);
        let b = rec("home/b.txt", 20);
        let f1 = fingerprint(&[&a, &b]);
        let f2 = fingerprint(&[&b, &a]); // different order, same set
        assert_eq!(f1, f2);
        assert_eq!(f1.len(), 16);
    }

    #[test]
    fn fingerprint_changes_when_size_changes() {
        let a = rec("home/a.txt", 10);
        let b = rec("home/b.txt", 20);
        let a2 = rec("home/a.txt", 11); // one byte different
        assert_ne!(fingerprint(&[&a, &b]), fingerprint(&[&a2, &b]));
    }
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
        filter: String::new(),
        filter_mode: false,
    };

    let mut terminal = ratatui::init();
    let res = run(&mut terminal, &mut app);
    ratatui::restore();
    res
}
