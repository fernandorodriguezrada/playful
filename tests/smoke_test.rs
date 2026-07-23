use std::path::Path;
use std::time::Duration;

fn with_temp_env<F: FnOnce()>(f: F) {
    let dir = tempfile::TempDir::new().unwrap();
    let config_dir = dir.path().join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    unsafe { std::env::set_var("XDG_CONFIG_HOME", config_dir.to_str().unwrap()) };
    f();
    // TempDir is dropped here, cleaning up
}

#[test]
fn test_config_save_and_load() {
    with_temp_env(|| {
        let music = Path::new("/tmp/test-music-for-config");
        let config = playful::config::Config {
            music_folder: music.to_path_buf(),
        };
        config.save().expect("save");
        let loaded = playful::config::Config::load().expect("load");
        assert_eq!(loaded.music_folder, music);
    });
}

#[test]
fn test_library_scan() {
    let dir = tempfile::TempDir::new().unwrap();
    let music_dir = dir.path().join("music");
    std::fs::create_dir_all(&music_dir).unwrap();

    let test_mp3 = music_dir.join("test.mp3");
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y", "-f", "lavfi",
            "-i", "sine=frequency=440:duration=3",
            "-metadata", "title=Test Track",
            "-metadata", "artist=Test Artist",
            "-metadata", "album=Test Album",
            "-ac", "1", "-ar", "44100",
            test_mp3.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "ffmpeg failed");

    let tracks = playful::library::scan_library(&music_dir);
    assert_eq!(tracks.len(), 1);
    let t = &tracks[0];
    assert_eq!(t.title, "Test Track");
    assert_eq!(t.artist, "Test Artist");
    assert_eq!(t.album, "Test Album");
    assert!(t.duration >= Duration::from_secs(2));
    assert_eq!(t.format, "MP3");
}
