use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::read_from_path;
use lofty::tag::ItemKey;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use walkdir::WalkDir;

pub fn extract_cover_art(path: &Path) -> Option<Vec<u8>> {
    let tagged_file = read_from_path(path).ok()?;
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.tags().first())?;
    let picture = tag.pictures().first()?;
    let data = picture.data();
    if data.is_empty() { None } else { Some(data.to_vec()) }
}

#[derive(Clone, Debug)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Duration,
    pub format: String,
    pub bitrate: u32,
    pub year: i32,
    pub sample_rate: u32,
    pub modified: String,
    pub channels: u8,
    pub genre: String,
    pub track_number: String,
    pub encoding: String,
    pub file_size: u64,
    #[allow(dead_code)]
    pub has_embedded_cover: bool,
    pub play_count: u32,
}

pub fn scan_library(path: &Path) -> Vec<Track> {
    let extensions = ["mp3", "flac", "m4a", "ogg", "wav", "wma", "aac", "opus"];

    let mut entries: Vec<_> = WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    entries.sort_by(|a, b| a.path().cmp(b.path()));

    entries
        .iter()
        .filter_map(|entry| {
            let ext = entry.path().extension()?.to_string_lossy().to_lowercase();
            if !extensions.contains(&ext.as_str()) {
                return None;
            }
            Some(parse_metadata(entry.path()))
        })
        .collect()
}

fn parse_metadata(path: &Path) -> Track {
    let mut title = String::new();
    let mut artist = String::new();
    let mut album = String::new();
    let mut duration = Duration::from_secs(0);
    let mut has_embedded_cover = false;
    let mut year = 0;
    let mut bitrate = 0u32;
    let mut sample_rate = 0u32;
    let modified;
    let mut channels = 0u8;
    let mut genre = String::new();
    let mut track_number = String::new();

    if let Ok(tagged_file) = read_from_path(path) {
        if let Some(tag) = tagged_file.primary_tag() {
            title = tag
                .get(ItemKey::TrackTitle)
                .and_then(|i| i.value().text())
                .unwrap_or("")
                .to_string();
            artist = tag
                .get(ItemKey::TrackArtist)
                .and_then(|i| i.value().text())
                .unwrap_or("")
                .to_string();
            album = tag
                .get(ItemKey::AlbumTitle)
                .and_then(|i| i.value().text())
                .unwrap_or("")
                .to_string();
            year = tag
                .get(ItemKey::Year)
                .and_then(|i| i.value().text())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            genre = tag
                .get(ItemKey::Genre)
                .and_then(|i| i.value().text())
                .unwrap_or("")
                .to_string();
            track_number = tag
                .get(ItemKey::TrackNumber)
                .and_then(|i| i.value().text())
                .unwrap_or("")
                .to_string();
            has_embedded_cover = !tag.pictures().is_empty();
        } else {
            for tag in tagged_file.tags() {
                title = tag
                    .get(ItemKey::TrackTitle)
                    .and_then(|i| i.value().text())
                    .unwrap_or("")
                    .to_string();
                artist = tag
                    .get(ItemKey::TrackArtist)
                    .and_then(|i| i.value().text())
                    .unwrap_or("")
                    .to_string();
                album = tag
                    .get(ItemKey::AlbumTitle)
                    .and_then(|i| i.value().text())
                    .unwrap_or("")
                    .to_string();
                year = tag
                    .get(ItemKey::Year)
                    .and_then(|i| i.value().text())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                if genre.is_empty() {
                    genre = tag
                        .get(ItemKey::Genre)
                        .and_then(|i| i.value().text())
                        .unwrap_or("")
                        .to_string();
                }
                if track_number.is_empty() {
                    track_number = tag
                        .get(ItemKey::TrackNumber)
                        .and_then(|i| i.value().text())
                        .unwrap_or("")
                        .to_string();
                }
                has_embedded_cover = !tag.pictures().is_empty();
                if !title.is_empty() {
                    break;
                }
            }
        }

        let properties = tagged_file.properties();
        duration = properties.duration();
        bitrate = properties.audio_bitrate().unwrap_or(0);
        sample_rate = properties.sample_rate().unwrap_or(0);
        channels = properties.channels().unwrap_or(0);
    }

    if title.is_empty() {
        title = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
    }

    if artist.is_empty() {
        artist = String::from("Unknown Artist");
    }

    let format = path
        .extension()
        .map(|e| e.to_string_lossy().to_uppercase())
        .unwrap_or_default();

    let encoding = match format.as_str() {
        "FLAC" | "WAV" | "AIFF" => "Lossless",
        "MP3" | "AAC" | "OGG" | "OPUS" | "WMA" => "Lossy",
        _ => "Unknown",
    }
    .to_string();

    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    modified = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .map(|t| {
            let since_epoch = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            let days = since_epoch / 86400;
            let mut remaining = days as i64;
            let mut y = 1970i64;
            loop {
                let days_in_year = if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) { 366 } else { 365 };
                if remaining < days_in_year { break; }
                remaining -= days_in_year;
                y += 1;
            }
            let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
            let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            let mut m = 0;
            for (i, &md) in month_days.iter().enumerate() {
                if remaining < md { m = i + 1; break; }
                remaining -= md;
            }
            if m == 0 { m = 12; remaining += month_days[11]; }
            format!("{}-{:02}-{:02}", y, m, remaining + 1)
        })
        .unwrap_or_default();

    Track {
        path: path.to_path_buf(),
        title,
        artist,
        album,
        duration,
        format,
        bitrate,
        year,
        sample_rate,
        modified,
        channels,
        genre,
        track_number,
        encoding,
        file_size,
        has_embedded_cover,
        play_count: 0,
    }
}

pub fn merge_play_counts(tracks: &mut [Track], play_counts: &HashMap<String, u32>) {
    for track in tracks.iter_mut() {
        let key = track.path.to_string_lossy().to_string();
        if let Some(count) = play_counts.get(&key) {
            track.play_count = *count;
        }
    }
}
