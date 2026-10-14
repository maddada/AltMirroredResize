//! The few preferences the app remembers, in `settings.ini` (`key = value`)
//! inside the per-user config folder.

use std::path::PathBuf;
use std::sync::RwLock;

pub const APP_NAME: &str = "AltMirroredResize";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub enabled: bool,
    /// Restart elevated at launch, so admin windows can be resized too.
    pub run_as_admin: bool,
    /// The first-run tip has been shown.
    pub welcomed: bool,
}

pub const DEFAULTS: Settings = Settings { enabled: true, run_as_admin: false, welcomed: false };

impl Default for Settings {
    fn default() -> Self {
        DEFAULTS
    }
}

static CURRENT: RwLock<Settings> = RwLock::new(DEFAULTS);

pub fn current() -> Settings {
    *CURRENT.read().unwrap_or_else(|e| e.into_inner())
}

/// Loads the saved settings (defaults if there are none).
pub fn load() {
    let loaded = std::fs::read_to_string(config_dir().join("settings.ini")).map(|t| from_text(&t)).unwrap_or_default();
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = loaded;
}

/// Changes the settings and saves them.
pub fn update(change: impl FnOnce(&mut Settings)) {
    let updated = {
        let mut s = CURRENT.write().unwrap_or_else(|e| e.into_inner());
        change(&mut s);
        *s
    };
    let dir = config_dir();
    let (path, tmp) = (dir.join("settings.ini"), dir.join("settings.ini.tmp"));
    let saved = std::fs::create_dir_all(&dir)
        .and_then(|_| std::fs::write(&tmp, to_text(&updated)))
        .and_then(|_| std::fs::rename(&tmp, &path));
    if let Err(e) = saved {
        super::log::write(&format!("couldn't save settings: {e}"));
    }
}

/// Per-user folder for the settings and the log.
pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));

    base.unwrap_or_else(std::env::temp_dir).join(APP_NAME)
}

fn to_text(s: &Settings) -> String {
    format!("enabled = {}\nrun_as_admin = {}\nwelcomed = {}\n", s.enabled, s.run_as_admin, s.welcomed)
}

fn from_text(text: &str) -> Settings {
    let mut s = DEFAULTS;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let Ok(value) = value.trim().parse::<bool>() else { continue };
        match key.trim() {
            "enabled" => s.enabled = value,
            "run_as_admin" => s.run_as_admin = value,
            "welcomed" => s.welcomed = value,
            _ => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let s = Settings { enabled: false, run_as_admin: true, welcomed: true };
        assert_eq!(from_text(&to_text(&s)), s);
    }

    #[test]
    fn ignores_junk_and_keeps_defaults_for_missing_keys() {
        let s = from_text("# comment\nenabled = nope\nrun_as_admin=true\nunknown = true\n");
        assert_eq!(s, Settings { run_as_admin: true, ..DEFAULTS });
    }
}
