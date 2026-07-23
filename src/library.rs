use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::read_from_path;
use lofty::tag::ItemKey;
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
    #[allow(dead_code)]
    pub has_embedded_cover: bool,
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
                has_embedded_cover = !tag.pictures().is_empty();
                if !title.is_empty() {
                    break;
                }
            }
        }

        let properties = tagged_file.properties();
        duration = properties.duration();
        bitrate = properties.audio_bitrate().unwrap_or(0);
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

    Track {
        path: path.to_path_buf(),
        title,
        artist,
        album,
        duration,
        format,
        bitrate,
        year,
        has_embedded_cover,
    }
}
