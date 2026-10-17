use std::fmt::Write as _;
use std::fs;

pub const CONFIG_FILE: &str = "autoclicker_settings.toml";

fn default_hotkey() -> String {
    "F6".to_string()
}

fn set_from_str<T: std::str::FromStr>(slot: &mut T, val: &str) {
    if let Ok(parsed) = val.parse() {
        *slot = parsed;
    }
}

#[derive(Clone)]
pub struct AppConfig {
    pub hours: u64,
    pub minutes: u64,
    pub seconds: u64,
    pub milliseconds: u64,
    pub repeat_infinite: bool,
    pub repeat_times: u64,
    pub mouse_button: u8,
    pub click_type: u8,
    pub use_custom_pos: bool,
    pub pos_x: i32,
    pub pos_y: i32,
    pub lang: u8,
    pub hotkey: String,
    pub always_on_top: bool,
    pub dark_mode: bool,
    pub autosave: bool,
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hours: 0,
            minutes: 0,
            seconds: 3,
            milliseconds: 0,
            repeat_infinite: true,
            repeat_times: 1,
            mouse_button: 0,
            click_type: 0,
            use_custom_pos: false,
            pos_x: 0,
            pos_y: 0,
            lang: 0,
            hotkey: default_hotkey(),
            always_on_top: false,
            dark_mode: true,
            autosave: true,
            window_x: None,
            window_y: None,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let mut cfg = Self::default();
        let Ok(text) = fs::read_to_string(CONFIG_FILE) else {
            return cfg;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, val)) = line.split_once('=') else {
                continue;
            };
            let val = val.trim().trim_matches('"');
            match key.trim() {
                "hours" => set_from_str(&mut cfg.hours, val),
                "minutes" => set_from_str(&mut cfg.minutes, val),
                "seconds" => set_from_str(&mut cfg.seconds, val),
                "milliseconds" => set_from_str(&mut cfg.milliseconds, val),
                "repeat_infinite" => set_from_str(&mut cfg.repeat_infinite, val),
                "repeat_times" => set_from_str(&mut cfg.repeat_times, val),
                "mouse_button" => set_from_str(&mut cfg.mouse_button, val),
                "click_type" => set_from_str(&mut cfg.click_type, val),
                "use_custom_pos" => set_from_str(&mut cfg.use_custom_pos, val),
                "pos_x" => set_from_str(&mut cfg.pos_x, val),
                "pos_y" => set_from_str(&mut cfg.pos_y, val),
                "lang" => set_from_str(&mut cfg.lang, val),
                "hotkey" => {
                    if !val.is_empty() {
                        cfg.hotkey = val.to_string();
                    }
                }
                "always_on_top" => set_from_str(&mut cfg.always_on_top, val),
                "dark_mode" => set_from_str(&mut cfg.dark_mode, val),
                "autosave" => set_from_str(&mut cfg.autosave, val),
                "window_x" => cfg.window_x = val.parse().ok(),
                "window_y" => cfg.window_y = val.parse().ok(),
                _ => {}
            }
        }
        cfg
    }

    pub fn save(&self) -> bool {
        let mut s = String::with_capacity(256);
        let _ = writeln!(s, "hours={}", self.hours);
        let _ = writeln!(s, "minutes={}", self.minutes);
        let _ = writeln!(s, "seconds={}", self.seconds);
        let _ = writeln!(s, "milliseconds={}", self.milliseconds);
        let _ = writeln!(s, "repeat_infinite={}", self.repeat_infinite);
        let _ = writeln!(s, "repeat_times={}", self.repeat_times);
        let _ = writeln!(s, "mouse_button={}", self.mouse_button);
        let _ = writeln!(s, "click_type={}", self.click_type);
        let _ = writeln!(s, "use_custom_pos={}", self.use_custom_pos);
        let _ = writeln!(s, "pos_x={}", self.pos_x);
        let _ = writeln!(s, "pos_y={}", self.pos_y);
        let _ = writeln!(s, "lang={}", self.lang);
        let _ = writeln!(s, "hotkey={}", self.hotkey);
        let _ = writeln!(s, "always_on_top={}", self.always_on_top);
        let _ = writeln!(s, "dark_mode={}", self.dark_mode);
        let _ = writeln!(s, "autosave={}", self.autosave);
        if let Some(x) = self.window_x {
            let _ = writeln!(s, "window_x={x}");
        }
        if let Some(y) = self.window_y {
            let _ = writeln!(s, "window_y={y}");
        }
        fs::write(CONFIG_FILE, s).is_ok()
    }

    pub fn saved_window_pos(&self) -> Option<(f32, f32)> {
        match (self.window_x, self.window_y) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() && x > -1000.0 && y > -1000.0 => {
                Some((x, y))
            }
            _ => None,
        }
    }

    pub fn total_interval_ms(&self) -> u64 {
        (self.hours.saturating_mul(3_600_000))
            .saturating_add(self.minutes.saturating_mul(60_000))
            .saturating_add(self.seconds.saturating_mul(1_000))
            .saturating_add(self.milliseconds)
            .max(1)
    }
}
