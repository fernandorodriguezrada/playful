use color_eyre::eyre::{eyre, Result};
use mpvipc::Mpv;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const FADE_LUA: &str = r#"
local at = nil
local function ka()
    if at then at:kill(); at = nil end
end
local function eoc(t) return 1 - (1 - t) ^ 3 end
local function fo()
    ka()
    local s = mp.get_property_number("volume")
    local n, i = 30, 0.01
    local st = 0
    local t = mp.add_periodic_timer(i, function()
        st = st + 1
        local ti = st / n
        mp.set_property_number("volume", s * (1 - eoc(ti)))
        if st >= n then
            mp.set_property_bool("pause", true)
            t:kill(); at = nil
        end
    end)
    t:resume(); at = t
end
local function fi(vol)
    ka()
    mp.set_property_number("volume", 0)
    mp.set_property_bool("pause", false)
    local n, i = 30, 0.01
    local st = 0
    local t = mp.add_periodic_timer(i, function()
        st = st + 1
        local ti = st / n
        mp.set_property_number("volume", vol * eoc(ti))
        if st >= n then
            t:kill(); at = nil
        end
    end)
    t:resume(); at = t
end
local function fa() ka() end
mp.register_script_message("playful_fade_out", fo)
mp.register_script_message("playful_fade_in", fi)
mp.register_script_message("playful_fade_abort", fa)
"#;

pub struct Player {
    mpv: Arc<Mutex<Option<Mpv>>>,
    socket_path: String,
    script_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub is_playing: bool,
    pub is_paused: bool,
    pub is_idle: bool,
    pub position: Duration,
    pub duration: Duration,
    pub current_track_path: Option<String>,
    pub volume: f64,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            is_playing: false,
            is_paused: false,
            is_idle: true,
            position: Duration::from_secs(0),
            duration: Duration::from_secs(0),
            current_track_path: None,
            volume: 50.0,
        }
    }
}

impl Player {
    pub fn new() -> Self {
        let pid = std::process::id();
        let socket_path = format!("/tmp/playful-mpv-{}", pid);
        let script_path = PathBuf::from(format!("/tmp/playful-fade-{}.lua", pid));
        Self {
            mpv: Arc::new(Mutex::new(None)),
            socket_path,
            script_path,
        }
    }

    pub fn start(&self) -> Result<()> {
        let _ = std::process::Command::new("pkill")
            .args(["-f", &format!("mpv.*{}", self.socket_path)])
            .output();

        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(&self.script_path);
        let _ = std::fs::write(&self.script_path, FADE_LUA);

        std::process::Command::new("mpv")
            .args([
                "--idle",
                "--no-terminal",
                &format!("--input-ipc-server={}", self.socket_path),
                "--no-video",
                "--audio-display=no",
                "--keep-open=no",
                "--ao=pulse",
                &format!("--script={}", self.script_path.display()),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        let mut retries = 0;
        while !std::path::Path::new(&self.socket_path).exists() && retries < 30 {
            std::thread::sleep(Duration::from_millis(100));
            retries += 1;
        }

        if !std::path::Path::new(&self.socket_path).exists() {
            return Err(eyre!("mpv socket was not created in time"));
        }

        let mpv = Mpv::connect(&self.socket_path)?;
        *self.mpv.lock().unwrap() = Some(mpv);

        Ok(())
    }

    pub fn load_and_play(&self, path: &Path) -> Result<()> {
        let mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            mpv.run_command_raw("loadfile", &[path.to_str().unwrap(), "replace"])?;
            mpv.set_property("pause", false)?;
        }
        Ok(())
    }

    pub fn toggle_pause(&self) -> Result<()> {
        let mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            let paused: bool = mpv.get_property("pause")?;
            mpv.set_property("pause", !paused)?;
        }
        Ok(())
    }

    pub fn set_pause(&self, paused: bool) -> Result<()> {
        let mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            mpv.set_property("pause", paused)?;
        }
        Ok(())
    }

    pub fn script_message(&self, name: &str, args: &[&str]) -> Result<()> {
        let mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            let mut full: Vec<&str> = vec![name];
            full.extend_from_slice(args);
            mpv.run_command_raw("script-message", &full)?;
        }
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            mpv.run_command_raw("stop", &[])?;
        }
        Ok(())
    }

    pub fn next(&self) -> Result<()> {
        let mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            mpv.run_command_raw("playlist-next", &[])?;
        }
        Ok(())
    }

    pub fn prev(&self) -> Result<()> {
        let mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            mpv.run_command_raw("playlist-prev", &[])?;
        }
        Ok(())
    }

    pub fn seek(&self, seconds: f64) {
        let mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            if let Ok(current) = mpv.get_property::<f64>("time-pos") {
                let _ = mpv.set_property("time-pos", (current + seconds).max(0.0));
            }
        }
    }

    pub fn set_volume(&self, volume: f64) -> Result<()> {
        let mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            mpv.set_property("volume", volume)?;
        }
        Ok(())
    }

    pub fn get_state(&self) -> PlayerState {
        let mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            let is_paused: bool = mpv.get_property("pause").unwrap_or(true);
            let is_idle: bool = mpv.get_property("idle-active").unwrap_or(true);
            let position: f64 = mpv.get_property("time-pos").unwrap_or(0.0);
            let duration: f64 = mpv.get_property("duration").unwrap_or(0.0);
            let volume: f64 = mpv.get_property("volume").unwrap_or(50.0);
            let path: Option<String> = mpv.get_property("path").ok();

            PlayerState {
                is_playing: !is_idle && !is_paused,
                is_paused,
                is_idle,
                position: Duration::from_secs_f64(position.max(0.0)),
                duration: Duration::from_secs_f64(duration.max(0.0)),
                current_track_path: path,
                volume,
            }
        } else {
            PlayerState::default()
        }
    }

    pub fn stop_mpv(&self) {
        let mut mpv = self.mpv.lock().unwrap();
        if let Some(ref mpv) = *mpv {
            let _ = mpv.run_command_raw("quit", &[]);
        }
        *mpv = None;
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_file(&self.script_path);
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop_mpv();
    }
}
