use crate::config::Config;
use crate::library::{merge_play_counts, scan_library, Track};
use crate::player::{Player, PlayerState};
use crate::ui;
use color_eyre::eyre::Result;
use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub enum AppScreen {
    Setup,
    Main,
}

pub struct App {
    pub screen: AppScreen,
    pub config: Config,
    pub library: Vec<Track>,
    pub selected_index: usize,
    pub player: Player,
    pub player_state: PlayerState,
    pub setup_path: String,
    pub status_message: Option<String>,
    pub status_time: Instant,
    pub change_folder_mode: bool,
    pub command_mode: bool,
    pub command_input: String,
    pub show_help: bool,
    pub marquee_offset: usize,
    marquee_counter: usize,
    prev_selected: usize,
    manual_stop: bool,
    pub cover_art: Option<Vec<u8>>,
    cover_art_decoded: Option<image::DynamicImage>,
    current_cover_path: Option<PathBuf>,
    art_needs_render: bool,
    last_art_area: Option<Rect>,
    played_tracks: HashSet<String>,
    fade_state: Option<FadeState>,
}

#[derive(Clone, Copy)]
enum FadeState {
    FadingOut { original_volume: f64, step: u8 },
    FadingIn { target_volume: f64, step: u8 },
}

impl App {
    pub fn new(config: Config) -> Self {
        let library = if !config.music_folder.as_os_str().is_empty() && config.music_folder.exists() {
            { let mut lib = scan_library(&config.music_folder); merge_play_counts(&mut lib, &config.play_counts); lib }
        } else {
            Vec::new()
        };

        let player = Player::new();

        let mut app = Self {
            screen: if config.music_folder.as_os_str().is_empty() || !config.music_folder.exists() {
                AppScreen::Setup
            } else {
                AppScreen::Main
            },
            config,
            library,
            selected_index: 0,
            player,
            player_state: PlayerState::default(),
            setup_path: String::new(),
            status_message: None,
            status_time: Instant::now(),
            change_folder_mode: false,
            command_mode: false,
            command_input: String::new(),
            show_help: false,
            marquee_offset: 0,
            marquee_counter: 0,
            prev_selected: 0,
            manual_stop: false,
            cover_art: None,
            cover_art_decoded: None,
            current_cover_path: None,
            art_needs_render: false,
            last_art_area: None,
            played_tracks: HashSet::new(),
            fade_state: None,
        };
        app.load_selected_cover();
        app
    }

    pub fn run(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        use crossterm::event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags, PopKeyboardEnhancementFlags};
        let _ = crossterm::execute!(stdout, PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        ));
        crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen, Show)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

        if let Err(e) = self.player.start() {
            eprintln!("Failed to start mpv: {}", e);
            crossterm::terminal::disable_raw_mode()?;
            crossterm::execute!(
                terminal.backend_mut(),
                crossterm::terminal::LeaveAlternateScreen
            )?;
            return Err(e);
        }

        let tick_rate = Duration::from_millis(100);
        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|f| ui::render(f, self))?;

            self.render_album_art();

            if self.status_message.is_some() && self.status_time.elapsed() > Duration::from_secs(4) {
                self.status_message = None;
            }

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_default();

            if event::poll(timeout)? {
                let ev = event::read()?;
                if let Event::Key(key) = ev {
                    if key.kind == KeyEventKind::Press {
                        if !self.handle_key(key)? {
                            break;
                        }
                    }
                }
            }

            let prev_state = self.player_state.clone();
            self.player_state = self.player.get_state();

            if !prev_state.is_idle
                && self.player_state.is_idle
                && !self.manual_stop
                && !prev_state.is_paused
            {
                self.play_next();
            }

            if self.player_state.is_playing {
                self.manual_stop = false;
            }

            self.check_play_count();
            self.process_fade();

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
                if self.selected_index != self.prev_selected {
                    self.marquee_offset = 0;
                    self.marquee_counter = 0;
                    self.prev_selected = self.selected_index;
                }
                self.marquee_counter += 1;
                if self.marquee_counter % 4 == 0 {
                    self.marquee_offset = self.marquee_offset.wrapping_add(1);
                }
            }
        }

        crossterm::terminal::disable_raw_mode()?;
        let _ = crossterm::execute!(
            terminal.backend_mut(),
            PopKeyboardEnhancementFlags,
            crossterm::terminal::LeaveAlternateScreen
        );

        Ok(())
    }

    fn handle_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match self.screen {
            AppScreen::Setup => self.handle_setup_key(key),
            AppScreen::Main => self.handle_main_key(key),
        }
    }

    fn handle_setup_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char(c) => {
                self.setup_path.push(c);
            }
            KeyCode::Backspace => {
                self.setup_path.pop();
            }
            KeyCode::Enter => {
                let trimmed = self.setup_path.trim();
                if trimmed.is_empty() {
                    self.set_status("Path cannot be empty!");
                    return Ok(true);
                }
                let path = PathBuf::from(trimmed);
                if path.exists() && path.is_dir() {
                    self.config.music_folder = path;
                    self.config.save()?;
                    self.rescan_library();
                    self.screen = AppScreen::Main;
                    let count = self.library.len();
                    self.set_status(&format!("Library loaded! {} tracks found", count));
                } else {
                    self.set_status("Invalid path! Please enter an existing directory");
                }
            }
            KeyCode::Esc => {
                return Ok(false);
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_main_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        if self.change_folder_mode {
            return self.handle_change_folder_key(key);
        }
        if self.command_mode {
            return self.handle_command_key(key);
        }
        if self.show_help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                self.show_help = false;
            }
            return Ok(true);
        }

        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.library.is_empty()
                    && self.selected_index < self.library.len().saturating_sub(1)
                {
                    self.selected_index += 1;
                    self.load_selected_cover();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let prev = self.selected_index;
                self.selected_index = self.selected_index.saturating_sub(1);
                if self.selected_index != prev {
                    self.load_selected_cover();
                }
            }
            KeyCode::PageDown => {
                self.selected_index = (self.selected_index + 10).min(self.library.len().saturating_sub(1));
                self.load_selected_cover();
            }
            KeyCode::PageUp => {
                self.selected_index = self.selected_index.saturating_sub(10);
                self.load_selected_cover();
            }
            KeyCode::Home => {
                self.selected_index = 0;
                self.load_selected_cover();
            }
            KeyCode::End => {
                self.selected_index = self.library.len().saturating_sub(1);
                self.load_selected_cover();
            }
            KeyCode::Enter => {
                if !self.library.is_empty() {
                    let is_current = self.player_state.current_track_path.as_deref()
                        .zip(self.library.get(self.selected_index))
                        .is_some_and(|(cur, track)| cur == track.path.to_string_lossy().as_ref());
                    if is_current {
                        self.manual_stop = false;
                        self.start_fade();
                    } else {
                        self.play_selected()?;
                    }
                }
            }
            KeyCode::Char(' ') => {
                if !self.library.is_empty() && self.player_state.is_idle {
                    self.play_selected()?;
                } else {
                    self.manual_stop = false;
                    self.start_fade();
                }
            }
            KeyCode::Right => {
                self.player.seek(5.0);
            }
            KeyCode::Left => {
                self.player.seek(-5.0);
            }
            KeyCode::Char('n') => {
                self.play_next();
            }
            KeyCode::Char('p') => {
                self.play_prev();
            }
            KeyCode::Char('s') => {
                self.manual_stop = true;
                self.player.stop()?;
                self.set_status("Stopped");
            }
            KeyCode::Char(';') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.command_mode = true;
                self.command_input.clear();
            }
            KeyCode::Char('c') => {
                self.change_folder_mode = true;
                self.setup_path.clear();
                self.clear_album_art();
            }
            KeyCode::Char('r') => {
                if !self.config.music_folder.as_os_str().is_empty() {
                    self.rescan_library();
                    self.selected_index = 0;
                    let count = self.library.len();
                    self.set_status(&format!("Library refreshed! {} tracks", count));
                }
            }
            KeyCode::Char('q') => {
                return Ok(false);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let vol = (self.player_state.volume + 5.0).min(100.0);
                self.player.set_volume(vol)?;
                self.set_status(&format!("Volume: {:.0}%", vol));
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                let vol = (self.player_state.volume - 5.0).max(0.0);
                self.player.set_volume(vol)?;
                self.set_status(&format!("Volume: {:.0}%", vol));
            }
            KeyCode::Char('.') => {
                self.player.seek(5.0);
            }
            KeyCode::Char(',') => {
                self.player.seek(-5.0);
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_change_folder_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char(c) => {
                self.setup_path.push(c);
            }
            KeyCode::Backspace => {
                self.setup_path.pop();
            }
            KeyCode::Enter => {
                let trimmed = self.setup_path.trim();
                let path = PathBuf::from(trimmed);
                if path.exists() && path.is_dir() {
                    self.config.music_folder = path;
                    self.config.save()?;
                    self.rescan_library();
                    self.selected_index = 0;
                    let count = self.library.len();
                    self.set_status(&format!("Music folder changed! {} tracks loaded", count));
                } else {
                    self.set_status("Invalid path!");
                }
                self.change_folder_mode = false;
                self.setup_path.clear();
            }
            KeyCode::Esc => {
                self.change_folder_mode = false;
                self.setup_path.clear();
            }
            _ => {}
        }
        Ok(true)
    }

    fn handle_command_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.command_mode = false;
                self.command_input.clear();
            }
            KeyCode::Char(c) => {
                self.command_input.push(c);
            }
            KeyCode::Backspace => {
                self.command_input.pop();
            }
            KeyCode::Enter => {
                let cmd = self.command_input.trim().to_lowercase();
                self.command_mode = false;
                self.command_input.clear();
                match cmd.as_str() {
                    "help" | "h" | "?" => {
                        self.show_help = true;
                    }
                    "refresh" | "r" => {
                        if !self.config.music_folder.as_os_str().is_empty() {
                            self.rescan_library();
                            self.selected_index = 0;
                            let count = self.library.len();
                            self.set_status(&format!("Library refreshed! {} tracks", count));
                        }
                    }
                    "quit" | "q" => return Ok(false),
                    "play" => {
                        if !self.library.is_empty() {
                            self.play_selected()?;
                        }
                    }
                    "pause" => {
                        if !self.player_state.is_idle {
                            self.manual_stop = false;
                            self.player.toggle_pause()?;
                        }
                    }
                    "stop" | "s" => {
                        self.manual_stop = true;
                        self.player.stop()?;
                        self.set_status("Stopped");
                    }
                    "next" | "n" => {
                        self.play_next();
                    }
                    "prev" | "p" => {
                        self.play_prev();
                    }
                    vol if vol.starts_with("volume ") || vol.starts_with("vol ") => {
                        let num_str = vol.split_whitespace().last().unwrap_or("50");
                        if let Ok(v) = num_str.parse::<f64>() {
                            let vol = v.clamp(0.0, 100.0);
                            self.player.set_volume(vol)?;
                            self.set_status(&format!("Volume: {:.0}%", vol));
                        }
                    }
                    _ => self.set_status(&format!("Unknown command: {}", cmd)),
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn play_selected(&mut self) -> Result<()> {
        if self.selected_index < self.library.len() {
            let track_path = self.library[self.selected_index].path.clone();
            let title = self.library[self.selected_index].title.clone();
            self.manual_stop = false;
            self.player.load_and_play(&track_path)?;
            self.set_status(&format!("♪ {}", title));
            self.load_cover_art(&track_path);
        }
        Ok(())
    }

    fn check_play_count(&mut self) {
        if !self.player_state.is_playing || self.player_state.is_idle || self.player_state.is_paused {
            return;
        }
        let track_path = match &self.player_state.current_track_path {
            Some(p) => p.clone(),
            None => return,
        };
        if self.played_tracks.contains(&track_path) {
            return;
        }
        let pos = self.player_state.position.as_secs_f64();
        let dur = self.player_state.duration.as_secs_f64();
        if dur <= 0.0 {
            return;
        }
        let threshold = 30.0f64.min(dur * 0.5);
        if pos >= threshold {
            self.played_tracks.insert(track_path.clone());
            if let Some(track) = self.library.iter_mut().find(|t| t.path.to_string_lossy().as_ref() == track_path) {
                track.play_count += 1;
                self.config.play_counts.insert(track_path, track.play_count);
                let _ = self.config.save();
            }
        }
    }

    fn start_fade(&mut self) {
        if self.player_state.is_idle {
            return;
        }
        if let Some(fade) = self.fade_state {
            self.fade_state = None;
            match fade {
                FadeState::FadingOut { original_volume, .. } => {
                    let _ = self.player.set_volume(original_volume);
                }
                FadeState::FadingIn { target_volume, .. } => {
                    let _ = self.player.set_volume(target_volume);
                    let _ = self.player.set_pause(true);
                }
            }
            return;
        }
        if self.player_state.is_paused {
            let target = self.player_state.volume;
            let _ = self.player.toggle_pause();
            let _ = self.player.set_volume(0.0);
            self.fade_state = Some(FadeState::FadingIn { target_volume: target, step: 0 });
        } else {
            self.fade_state = Some(FadeState::FadingOut { original_volume: self.player_state.volume, step: 0 });
        }
    }

    fn process_fade(&mut self) {
        let Some(fade) = &self.fade_state else { return };
        match *fade {
            FadeState::FadingOut { original_volume, step } => {
                let next = step + 1;
                let fraction = 1.0 - next as f64 / 3.0;
                let vol = (original_volume * fraction).max(0.0);
                let _ = self.player.set_volume(vol);
                if next >= 3 {
                    let _ = self.player.set_pause(true);
                    self.fade_state = None;
                } else {
                    self.fade_state = Some(FadeState::FadingOut { original_volume, step: next });
                }
            }
            FadeState::FadingIn { target_volume, step } => {
                let next = step + 1;
                let fraction = next as f64 / 3.0;
                let vol = (target_volume * fraction).min(target_volume);
                let _ = self.player.set_volume(vol);
                if next >= 3 {
                    self.fade_state = None;
                } else {
                    self.fade_state = Some(FadeState::FadingIn { target_volume, step: next });
                }
            }
        }
    }

    fn load_selected_cover(&mut self) {
        let path = self.library.get(self.selected_index).map(|t| t.path.clone());
        if let Some(p) = path {
            self.load_cover_art(&p);
        }
    }

    fn load_cover_art(&mut self, path: &Path) {
        if self.current_cover_path.as_deref() == Some(path) {
            return;
        }
        self.cover_art = crate::library::extract_cover_art(path);
        let decoded = self.cover_art.as_ref()
            .and_then(|data| image::load_from_memory(data).ok());
        self.cover_art = None;
        self.cover_art_decoded = decoded.map(|img| {
            let max_dim = 300u32;
            let w = img.width();
            let h = img.height();
            let resized = if w > max_dim || h > max_dim {
                let ratio = if w > h { max_dim as f64 / w as f64 } else { max_dim as f64 / h as f64 };
                let nw = (w as f64 * ratio).round() as u32;
                let nh = (h as f64 * ratio).round() as u32;
                img.resize_exact(nw, nh, image::imageops::FilterType::Lanczos3)
            } else {
                img
            };
            round_corners(resized, 0.08)
        });
        self.current_cover_path = Some(path.to_path_buf());
        self.art_needs_render = true;
    }

    fn play_next(&mut self) {
        if self.selected_index < self.library.len().saturating_sub(1) {
            self.selected_index += 1;
            let track_path = self.library[self.selected_index].path.clone();
            self.manual_stop = false;
            let _ = self.player.load_and_play(&track_path);
            self.load_cover_art(&track_path);
        }
    }

    fn play_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            let track_path = self.library[self.selected_index].path.clone();
            self.manual_stop = false;
            let _ = self.player.load_and_play(&track_path);
            self.load_cover_art(&track_path);
        }
    }

    pub fn has_cover_art(&self) -> bool {
        self.cover_art_decoded.is_some()
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some(msg.to_string());
        self.status_time = Instant::now();
    }

    fn rescan_library(&mut self) {
        self.library = scan_library(&self.config.music_folder);
        merge_play_counts(&mut self.library, &self.config.play_counts);
    }

    fn clear_album_art(&self) {
        use std::io::Write;
        let _ = io::stdout().write_all(b"\x1b_Ga=d\x1b\\");
        let _ = io::stdout().flush();
    }

    fn render_album_art(&mut self) {
        let area = crate::ui::get_album_art_area();
        let has_area = area.is_some_and(|r| r.width > 4 && r.height > 2);
        if !has_area {
            if self.last_art_area.is_some() && self.last_art_area != Some(Rect::new(0, 0, 0, 0)) {
                self.clear_album_art();
                self.last_art_area = Some(Rect::new(0, 0, 0, 0));
            }
            self.art_needs_render = false;
            return;
        }
        let img = match &self.cover_art_decoded {
            Some(img) => img,
            None => {
                if self.last_art_area.is_some() {
                    self.clear_album_art();
                    self.last_art_area = None;
                }
                self.art_needs_render = false;
                return;
            }
        };
        let rect = area.unwrap();

        let area_changed = self.last_art_area != Some(rect);
        if !self.art_needs_render && !area_changed {
            return;
        }

        self.last_art_area = Some(rect);
        self.art_needs_render = false;

        let art_w = rect.width.saturating_sub(2).max(4) as u32;
        let art_h = rect.height.saturating_sub(2).max(4) as u32;
        let config = viuer::Config {
            x: rect.x + 1,
            y: rect.y as i16 + 1,
            width: Some(art_w.min(art_h.saturating_mul(2))),
            height: None,
            absolute_offset: true,
            use_kitty: true,
            transparent: true,
            ..Default::default()
        };

        let _ = viuer::print(img, &config);
    }
}

fn round_corners(img: image::DynamicImage, radius_pct: f64) -> image::DynamicImage {
    let mut rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let r = ((w.min(h) as f64) * radius_pct).round() as u32;
    let r = r.max(1);
    let r_sq = r * r;

    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = if x < r && y < r {
                (r - 1 - x, r - 1 - y)
            } else if x >= w - r && y < r {
                (x - (w - r), r - 1 - y)
            } else if x < r && y >= h - r {
                (r - 1 - x, y - (h - r))
            } else if x >= w - r && y >= h - r {
                (x - (w - r), y - (h - r))
            } else {
                continue;
            };
            if dx * dx + dy * dy > r_sq {
                rgba.get_pixel_mut(x, y).0[3] = 0;
            }
        }
    }

    image::DynamicImage::ImageRgba8(rgba)
}
