use crate::app::{App, AppScreen};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap, *};
use std::sync::OnceLock;
use std::time::Duration;

static ALBUM_ART_AREA: OnceLock<std::sync::Mutex<Option<Rect>>> = OnceLock::new();

pub fn get_album_art_area() -> Option<Rect> {
    let lock = ALBUM_ART_AREA.get_or_init(|| std::sync::Mutex::new(None));
    *lock.lock().unwrap()
}

fn set_album_art_area(rect: Rect) {
    let lock = ALBUM_ART_AREA.get_or_init(|| std::sync::Mutex::new(None));
    *lock.lock().unwrap() = Some(rect);
}

const RAINBOW_COLORS: [Color; 5] = [K, P, C, T, D];

const D2: Color = Color::Rgb(130, 145, 165);
const K: Color = Color::Rgb(255, 183, 211);
const P: Color = Color::Rgb(223, 184, 255);
const C: Color = Color::Rgb(177, 211, 254);
const T: Color = Color::Rgb(163, 241, 203);
const D: Color = Color::Rgb(97, 114, 133);

pub fn render(f: &mut Frame, app: &App) {
    match app.screen {
        AppScreen::Setup => render_setup(f, app),
        AppScreen::Main => render_main(f, app),
    }
}

fn render_setup(f: &mut Frame, app: &App) {
    let area = f.area();

    let overlay = Block::default()
        .style(Style::default().bg(Color::Rgb(20, 20, 28)));
    f.render_widget(overlay, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .horizontal_margin(area.width / 5)
        .vertical_margin(area.height / 3)
        .split(area);

    let title = "♪ Playful - Terminal Music Player ♪";
    let mut spans = Vec::new();
    for (i, ch) in title.chars().enumerate() {
        let color = RAINBOW_COLORS[i % RAINBOW_COLORS.len()];
        spans.push(Span::styled(
            ch.to_string(),
            Style::default()
                .fg(color)
                .add_modifier(Modifier::BOLD),
        ));
    }
    let rainbow_title = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    f.render_widget(rainbow_title, chunks[0]);

    let prompt = Paragraph::new("Enter your music folder path:")
        .style(Style::default().fg(C))
        .alignment(Alignment::Center);
    f.render_widget(prompt, chunks[2]);

    let input_rect = chunks[3];
    let input = Paragraph::new(app.setup_path.as_str())
        .style(Style::default().fg(C))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(D)),
        );
    f.render_widget(input, input_rect);

    let help = Paragraph::new("Enter to confirm  ·  Esc to quit")
        .style(Style::default().fg(D))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[4]);

    let right_edge = input_rect.x + input_rect.width.saturating_sub(2);
    let cursor_x = input_rect.x + 1 + app.setup_path.len() as u16;
    f.set_cursor_position((cursor_x.min(right_edge), input_rect.y + 1));
}

fn render_main(f: &mut Frame, app: &App) {
    let area = f.area();

    if app.show_help {
        set_album_art_area(Rect::new(0, 0, 0, 0));
        render_help(f, area);
        return;
    }

    if app.change_folder_mode {
        set_album_art_area(Rect::new(0, 0, 0, 0));
        render_change_folder_fullscreen(f, area, app);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(area);

    render_header(f, chunks[0]);
    render_content(f, chunks[1], app);

    if app.command_mode {
        render_command_input(f, chunks[2], app);
    } else {
        render_now_playing(f, chunks[2], app);
    }
}

fn render_command_input(f: &mut Frame, area: Rect, app: &App) {
    let input = Paragraph::new(app.command_input.as_str())
        .style(Style::default().fg(C))
        .block(
            Block::default()
                .title(" Command ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(D)),
        );
    f.render_widget(input, area);
    f.set_cursor_position((area.x + 1 + app.command_input.len() as u16, area.y + 1));
}

fn render_header(f: &mut Frame, area: Rect) {
    let title = "♪ Playful - Terminal Music Player ♪";

    let mut spans = Vec::new();
    for (i, ch) in title.chars().enumerate() {
        let color = RAINBOW_COLORS[i % RAINBOW_COLORS.len()];
        spans.push(Span::styled(
            ch.to_string(),
            Style::default()
                .fg(color)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let text = Text::from(Line::from(spans));
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::BOTTOM))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn render_content(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    render_track_list(f, chunks[0], app);

    let has_art = app.has_cover_art();
    let right_chunks = if has_art {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(1)])
            .split(chunks[1])
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(0), Constraint::Min(1)])
            .split(chunks[1])
    };

    if has_art {
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(14)])
            .split(right_chunks[0]);
        render_file_details(f, top_chunks[0], app);
        set_album_art_area(top_chunks[1]);
    } else {
        set_album_art_area(Rect::new(0, 0, 0, 0));
    }
    render_info_panel(f, right_chunks[1], app);
}

pub fn render_help(f: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    let title = "  ♪ Playful  ";
    let mut title_spans: Vec<Span> = title.chars().enumerate()
        .map(|(i, c)| Span::styled(c.to_string(), Style::default().fg(RAINBOW_COLORS[i % 5]).add_modifier(Modifier::BOLD)))
        .collect();
    title_spans.push(Span::styled("  ", Style::default()));
    title_spans.push(Span::styled("—", Style::default().fg(D)));
    title_spans.push(Span::styled("  ", Style::default()));
    title_spans.push(Span::styled("terminal music player", Style::default().add_modifier(Modifier::ITALIC).fg(D2)));
    lines.push(Line::from(title_spans));
    lines.push(Line::from(Span::raw("")));

    let sections: &[(&str, &[(&str, &str)])] = &[
        ("Keys", &[
            ("↑↓  /  jk", "Navigate track list"),
            ("Enter",     "Play selected track"),
            ("Space",     "Play / Pause"),
            ("n  /  p",   "Next / Previous track"),
            ("s",          "Stop playback"),
            ("+  /  -",   "Volume up / down"),
            (".  /  ,",   "Seek forward / back 5s"),
            ("c",          "Change music folder"),
            ("r",          "Refresh library"),
            ("Ctrl+;",     "Open command palette"),
            ("q",          "Quit"),
        ]),
        ("Commands", &[
            ("help",             "Show this help screen"),
            ("quit",             "Exit the application"),
            ("refresh",          "Rescan music library"),
            ("play",             "Play selected track"),
            ("pause",            "Play / Pause"),
            ("stop",             "Stop playback"),
            ("next",             "Next track"),
            ("prev",             "Previous track"),
            ("volume <0-100>",   "Set volume level"),
        ]),
    ];

    let col_width = 32usize;

    for (section_name, items) in sections {
        let sec: Vec<Span> = section_name.chars().enumerate()
            .map(|(i, c)| Span::styled(c.to_string(), Style::default().fg(RAINBOW_COLORS[i % 5]).add_modifier(Modifier::BOLD)))
            .collect();
        lines.push(Line::from(sec));
        for (cmd, desc) in *items {
            let padded = format!("{:<1$}", cmd, col_width);
            lines.push(Line::from(vec![
                Span::styled(format!("    {}", padded), Style::default().fg(C)),
                Span::styled(*desc, Style::default().fg(D)),
            ]));
        }
        lines.push(Line::from(Span::raw("")));
    }

    lines.push(Line::from(vec![Span::styled(
        "  Esc or q to close",
        Style::default().fg(D),
    )]));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(paragraph, area);
}

fn render_change_folder_fullscreen(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .horizontal_margin(area.width / 5)
        .vertical_margin(area.height / 3)
        .split(area);

    let title = Paragraph::new("Change Music Folder")
        .style(Style::default().fg(C).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let current = app.config.music_folder.display().to_string();
    let current_label = Paragraph::new(Line::from(vec![
        Span::styled("Current: ", Style::default().fg(D2)),
        Span::styled(current, Style::default().fg(D)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(current_label, chunks[2]);

    let input_rect = chunks[3];

    let input = Paragraph::new(app.setup_path.as_str())
        .style(Style::default().fg(C))
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(D)));
    f.render_widget(input, input_rect);

    let help = Paragraph::new("Enter to confirm  ·  Esc to cancel")
        .style(Style::default().fg(D))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[4]);

    let right_edge = input_rect.x + input_rect.width.saturating_sub(2);
    let cursor_x = input_rect.x + 1 + app.setup_path.len() as u16;
    f.set_cursor_position((cursor_x.min(right_edge), input_rect.y + 1));
}

fn marquee_text(text: &str, offset: usize, _cell_width: usize) -> String {
    if text.chars().count() <= 16 {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len() + 8 + 8;
    let cursor = offset % total;
    if cursor < 8 {
        return text.to_string();
    }
    let scroll = cursor - 8;
    if scroll >= chars.len() {
        return text.to_string();
    }
    chars[scroll..].iter().collect()
}

fn render_track_list(f: &mut Frame, area: Rect, app: &App) {
    if app.library.is_empty() {
        let msg = Paragraph::new("No music files found.\nPress 'c' to set your music folder.")
            .style(Style::default().fg(D))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Library ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            );
        f.render_widget(msg, area);
        return;
    }

    let items_len = app.library.len();
    let header_cells = ["Track", "Artist", "Album"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(D2).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells);

    let inner_w = area.width.saturating_sub(2);
    let track_cw = (inner_w as f64 * 0.50).floor() as usize;
    let artist_cw = (inner_w as f64 * 0.25).floor() as usize;
    let album_cw = (inner_w as f64 * 0.25).floor() as usize;
    let track_cw = track_cw.saturating_sub(6);
    let artist_cw = artist_cw.saturating_sub(4);
    let album_cw = album_cw.saturating_sub(4);

    let rows: Vec<Row> = app
        .library
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let is_selected = i == app.selected_index;
            let style = if is_selected {
                Style::default()
                    .fg(K)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let title_text = if is_selected {
                marquee_text(&track.title, app.marquee_offset, track_cw)
            } else {
                track.title.clone()
            };

            let artist_text = if is_selected {
                marquee_text(&track.artist, app.marquee_offset, artist_cw)
            } else {
                track.artist.clone()
            };

            let album_text = if is_selected {
                marquee_text(&track.album, app.marquee_offset, album_cw)
            } else {
                track.album.clone()
            };

            Row::new(vec![
                Cell::from(title_text),
                Cell::from(artist_text),
                Cell::from(album_text),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(format!(
                " Library ({} {}) ",
                items_len,
                if items_len == 1 { "track" } else { "tracks" }
            ))
            .title_style(Style::default().fg(C))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    )
    .row_highlight_style(
        Style::default()
            .fg(K)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▸ ");

    let mut table_state = TableState::default();
    table_state.select(Some(app.selected_index));
    f.render_stateful_widget(table, area, &mut table_state);
}

fn render_info_panel(f: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();

    if let Some(track) = app.library.get(app.selected_index) {
        lines.push(Line::from(vec![Span::styled(
            "Track Info",
            Style::default()
                .fg(P)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(vec![
            Span::styled("Title:   ", Style::default().fg(D2)),
            Span::styled(&track.title, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Artist:  ", Style::default().fg(D2)),
            Span::styled(&track.artist, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Album:   ", Style::default().fg(D2)),
            Span::styled(&track.album, Style::default().fg(Color::White)),
        ]));
        if !track.genre.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Genre:   ", Style::default().fg(D2)),
                Span::styled(&track.genre, Style::default().fg(Color::White)),
            ]));
        }
        if !track.track_number.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Track:   ", Style::default().fg(D2)),
                Span::styled(&track.track_number, Style::default().fg(Color::White)),
            ]));
        }
        lines.push(Line::from(vec![
            Span::styled("Length:  ", Style::default().fg(D2)),
            Span::styled(
                format_duration(track.duration),
                Style::default().fg(Color::White),
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "No tracks in library",
            Style::default().fg(D2),
        )));
    }

    lines.push(Line::from(vec![Span::styled(
        "Controls",
        Style::default()
            .fg(P)
            .add_modifier(Modifier::BOLD),
    )]));
    let shortcuts = [
        ("↑↓/jk", "Navigate"),
        ("Enter", "Play track"),
        ("Space", "Play/Pause"),
        ("n/p", "Next/Prev"),
        ("s", "Stop"),
        ("+/-", "Volume"),
        ("./,", "Seek ±5s"),
        ("c", "Change folder"),
        ("r", "Refresh"),
        ("q", "Quit"),
        ("Ctrl+;",  "Command palette"),
    ];
    for (key, action) in &shortcuts {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<8}", key),
                Style::default().fg(C),
            ),
            Span::styled(*action, Style::default().fg(D2)),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" Info ")
                .title_style(Style::default().fg(C))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );

    f.render_widget(paragraph, area);
}

fn format_sample_rate(hz: u32) -> String {
    if hz == 0 { return String::new() }
    format!("{:.1} kHz", hz as f64 / 1000.0)
}

fn format_channels(n: u8) -> String {
    match n {
        1 => "Mono".into(),
        2 => "Stereo".into(),
        _ if n > 0 => format!("{} ch", n),
        _ => String::new(),
    }
}

fn format_file_size(bytes: u64) -> String {
    if bytes == 0 { return String::new() }
    let units = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size > 1024.0 && unit < 3 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", size, units[unit])
}

fn render_file_details(f: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();

    let plabel = |s: &'static str| Span::styled(s, Style::default().fg(D2));

    if let Some(track) = app.library.get(app.selected_index) {
        lines.push(Line::from(vec![plabel("Format: "), Span::styled(&track.format, Style::default().fg(Color::White))]));
        if track.bitrate > 0 {
            lines.push(Line::from(vec![
                plabel("Bitrate: "),
                Span::styled(format!("{}k", track.bitrate), Style::default().fg(Color::White)),
            ]));
        }
        if track.sample_rate > 0 {
            lines.push(Line::from(vec![
                plabel("Sample:  "),
                Span::styled(format_sample_rate(track.sample_rate), Style::default().fg(Color::White)),
            ]));
        }
        if !track.modified.is_empty() {
            lines.push(Line::from(vec![
                plabel("Date:    "),
                Span::styled(&track.modified, Style::default().fg(Color::White)),
            ]));
        }
        if track.channels > 0 {
            lines.push(Line::from(vec![
                plabel("Channels:"),
                Span::styled(format_channels(track.channels), Style::default().fg(Color::White)),
            ]));
        }
        if !track.encoding.is_empty() && track.encoding != "Unknown" {
            lines.push(Line::from(vec![
                plabel("Type:    "),
                Span::styled(track.encoding.clone(), Style::default().fg(Color::White)),
            ]));
        }
        if track.year > 0 {
            lines.push(Line::from(vec![
                plabel("Year:    "),
                Span::styled(track.year.to_string(), Style::default().fg(Color::White)),
            ]));
        }
        if track.file_size > 0 {
            lines.push(Line::from(vec![
                plabel("Size:    "),
                Span::styled(format_file_size(track.file_size), Style::default().fg(Color::White)),
            ]));
        }
    }

    if lines.is_empty() {
        return;
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" File Details ")
                .title_style(Style::default().fg(C))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );

    f.render_widget(paragraph, area);
}

fn render_now_playing(f: &mut Frame, area: Rect, app: &App) {
    let state = &app.player_state;

    let track_title = state
        .current_track_path
        .as_deref()
        .and_then(|path| {
            app.library
                .iter()
                .find(|t| t.path.to_string_lossy().as_ref() == path)
        })
        .map(|t| format!("{} - {}", t.artist, t.title))
        .unwrap_or_else(|| {
            state
                .current_track_path
                .as_deref()
                .and_then(|p| {
                    std::path::Path::new(p)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                })
                .unwrap_or_else(|| "Nothing Playing".to_string())
        });

    let status_text = if state.is_playing {
        "Now Playing"
    } else if state.is_paused {
        "Paused"
    } else if !state.is_idle {
        "Now Playing"
    } else {
        "No Playback"
    };

    let volume = format!("VOL {:3.0}%", state.volume);
    let pos_str = format_duration(state.position);
    let dur_str = format_duration(state.duration);

    let bar_width = (area.width as usize).saturating_sub(25).max(10);
    let progress = if state.duration > Duration::from_secs(0) {
        let ratio = state.position.as_secs_f64() / state.duration.as_secs_f64();
        let filled = (ratio * bar_width as f64).round() as usize;
        let filled = filled.min(bar_width);
        let bar = "━".repeat(filled) + "⬤" + &"━".repeat(bar_width.saturating_sub(filled + 1));
        bar
    } else {
        "━".repeat(bar_width)
    };

    let mut spans_top = vec![
        Span::styled(
            format!(" {} ", status_text),
            Style::default()
                .fg(T)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(D)),
        Span::styled(
            &track_title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if let Some(ref msg) = app.status_message {
        spans_top.push(Span::raw("  "));
        spans_top.push(Span::styled(
            format!("[{}]", msg),
            Style::default().fg(K),
        ));
    }

    let line1 = Line::from(spans_top);

    let line2 = Line::from(vec![
        Span::styled(pos_str, Style::default().fg(C)),
        Span::styled(" ", Style::default()),
        Span::styled(progress, Style::default().fg(C)),
        Span::styled(" ", Style::default()),
        Span::styled(dur_str, Style::default().fg(C)),
        Span::styled(" │ ", Style::default().fg(D)),
        Span::styled(volume, Style::default().fg(T)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let paragraph = Paragraph::new(Text::from(vec![
        line1,
        line2,
    ])).block(block);
    f.render_widget(paragraph, area);
}

pub fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}
