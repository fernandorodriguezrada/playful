use crate::app::{App, AppScreen};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::widgets::*;
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

const RAINBOW_COLORS: [Color; 5] = [
    Color::Rgb(163, 241, 203),
    Color::Rgb(177, 211, 254),
    Color::Rgb(223, 184, 255),
    Color::Rgb(255, 183, 211),
    Color::Rgb(97, 114, 133),
];

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
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(prompt, chunks[2]);

    let input_rect = chunks[3];
    let input = Paragraph::new(app.setup_path.as_str())
        .style(Style::default().fg(Color::Rgb(177, 211, 254)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(97, 114, 133))),
        );
    f.render_widget(input, input_rect);

    let help = Paragraph::new("Enter to confirm  ·  Esc to quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[4]);

    let right_edge = input_rect.x + input_rect.width.saturating_sub(2);
    let cursor_x = input_rect.x + 1 + app.setup_path.len() as u16;
    f.set_cursor_position((cursor_x.min(right_edge), input_rect.y + 1));
}

fn render_main(f: &mut Frame, app: &App) {
    let area = f.area();

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
    render_now_playing(f, chunks[2], app);
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
            .constraints([Constraint::Length(14), Constraint::Min(1)])
            .split(right_chunks[0]);
        set_album_art_area(top_chunks[0]);
        render_file_details(f, top_chunks[1], app);
    } else {
        set_album_art_area(Rect::new(0, 0, 0, 0));
    }
    render_info_panel(f, right_chunks[1], app);
}

fn render_change_folder_fullscreen(f: &mut Frame, area: Rect, app: &App) {
    let overlay = Block::default()
        .style(Style::default().bg(Color::Rgb(20, 20, 28)));
    f.render_widget(overlay, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .horizontal_margin(area.width / 5)
        .vertical_margin(area.height / 3)
        .split(area);

    let title = Paragraph::new("Change Music Folder")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let input_rect = chunks[2];

    let input = Paragraph::new(app.setup_path.as_str())
        .style(Style::default().fg(Color::Rgb(177, 211, 254)))
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(97, 114, 133))));
    f.render_widget(input, input_rect);

    let help = Paragraph::new("Enter to confirm  ·  Esc to cancel")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[3]);

    let right_edge = input_rect.x + input_rect.width.saturating_sub(2);
    let cursor_x = input_rect.x + 1 + app.setup_path.len() as u16;
    f.set_cursor_position((cursor_x.min(right_edge), input_rect.y + 1));
}

fn render_track_list(f: &mut Frame, area: Rect, app: &App) {
    if app.library.is_empty() {
        let msg = Paragraph::new("No music files found.\nPress 'c' to set your music folder.")
            .style(Style::default().fg(Color::DarkGray))
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

    let items: Vec<ListItem> = app
        .library
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let is_selected = i == app.selected_index;
            let prefix = if is_selected { "▸ " } else { "  " };
            let title = track.title.as_str();

            let artist = if track.artist == "Unknown Artist" && !track.album.is_empty() {
                track.album.as_str()
            } else {
                track.artist.as_str()
            };

            let dur = format_duration(track.duration);
            let content = format!("{}{}  ·  {}  [{}]", prefix, title, artist, dur);

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let items_len = items.len();
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    " Library ({} {}) ",
                    items_len,
                    if items_len == 1 { "track" } else { "tracks" }
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_index));
    f.render_stateful_widget(list, area, &mut list_state);
}

fn render_info_panel(f: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();

    if let Some(track) = app.library.get(app.selected_index) {
        lines.push(Line::from(vec![Span::styled(
            "Track Info",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(vec![
            Span::styled("Title:   ", Style::default().fg(Color::Rgb(150, 150, 170))),
            Span::styled(&track.title, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Artist:  ", Style::default().fg(Color::Rgb(150, 150, 170))),
            Span::styled(&track.artist, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Album:   ", Style::default().fg(Color::Rgb(150, 150, 170))),
            Span::styled(&track.album, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Length:  ", Style::default().fg(Color::Rgb(150, 150, 170))),
            Span::styled(
                format_duration(track.duration),
                Style::default().fg(Color::White),
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "No tracks in library",
            Style::default().fg(Color::Rgb(150, 150, 170)),
        )));
    }

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(vec![Span::styled(
        "Controls",
        Style::default()
            .fg(Color::Cyan)
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
    ];
    for (key, action) in &shortcuts {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<8}", key),
                Style::default().fg(Color::Rgb(177, 211, 254)),
            ),
            Span::styled(*action, Style::default().fg(Color::Rgb(150, 150, 170))),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Info ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );

    f.render_widget(paragraph, area);
}

fn render_file_details(f: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();

    if let Some(track) = app.library.get(app.selected_index) {
        lines.push(Line::from(vec![
            Span::styled("Format: ", Style::default().fg(Color::Rgb(150, 150, 170))),
            Span::styled(&track.format, Style::default().fg(Color::White)),
        ]));
        if track.bitrate > 0 {
            lines.push(Line::from(vec![
                Span::styled("Bitrate: ", Style::default().fg(Color::Rgb(150, 150, 170))),
                Span::styled(format!("{}k", track.bitrate), Style::default().fg(Color::White)),
            ]));
        }
        if track.year > 0 {
            lines.push(Line::from(vec![
                Span::styled("Year:    ", Style::default().fg(Color::Rgb(150, 150, 170))),
                Span::styled(track.year.to_string(), Style::default().fg(Color::White)),
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

    let status_icon = if state.is_playing {
        "▶"
    } else if state.is_paused {
        "⏸"
    } else if !state.is_idle {
        "▶"
    } else {
        "⏹"
    };

    let volume = format!("VOL {:3.0}%", state.volume);
    let pos_str = format_duration(state.position);
    let dur_str = format_duration(state.duration);

    let bar_width = (area.width as usize).saturating_sub(25).max(10);
    let progress = if state.duration > Duration::from_secs(0) {
        let ratio = state.position.as_secs_f64() / state.duration.as_secs_f64();
        let filled = (ratio * bar_width as f64).round() as usize;
        let filled = filled.min(bar_width);
        let bar = "━".repeat(filled) + "●" + &"━".repeat(bar_width.saturating_sub(filled + 1));
        bar
    } else {
        "━".repeat(bar_width)
    };

    let mut spans_top = vec![
        Span::styled(
            format!(" {} ", status_icon),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
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
            Style::default().fg(Color::Yellow),
        ));
    }

    let line1 = Line::from(spans_top);

    let line2 = Line::from(vec![
        Span::styled(pos_str, Style::default().fg(Color::Cyan)),
        Span::styled(" ", Style::default()),
        Span::styled(progress, Style::default().fg(Color::Cyan)),
        Span::styled(" ", Style::default()),
        Span::styled(dur_str, Style::default().fg(Color::Cyan)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(volume, Style::default().fg(Color::Green)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let paragraph = Paragraph::new(Text::from(vec![line1, line2])).block(block);
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
