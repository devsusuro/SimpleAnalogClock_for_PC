use std::{fmt::Write, fs, io, path::PathBuf};

use eframe::egui::{Pos2, pos2};
use serde::{Serialize, de::DeserializeOwned};

use crate::{Opacity, Settings, language::Language};

pub struct SettingsFile {
    path: PathBuf,
    had_file: bool,
}

impl SettingsFile {
    pub fn beside_executable() -> io::Result<Self> {
        let path = std::env::current_exe()?.with_extension("ini");
        let had_file = path.try_exists()?;
        Ok(Self { path, had_file })
    }

    pub fn load(&self) -> io::Result<Settings> {
        match fs::read_to_string(&self.path) {
            Ok(text) => Ok(decode(&text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(error) => Err(error),
        }
    }

    pub fn save(&mut self, settings: &Settings, explicit: bool) -> io::Result<()> {
        // Deleting the file while the clock runs must also reset the next launch.
        if !explicit && self.had_file && !self.path.try_exists()? {
            return Ok(());
        }
        let temporary = self
            .path
            .with_extension(format!("ini.{}.tmp", std::process::id()));
        let result = fs::write(&temporary, encode(settings))
            .and_then(|()| fs::rename(&temporary, &self.path));
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        } else {
            self.had_file = true;
        }
        result
    }
}

fn decode(text: &str) -> Settings {
    let mut settings = Settings::default();
    let (mut x, mut y) = (None, None);
    let mut clock_section = false;
    for line in text.trim_start_matches('\u{feff}').lines().map(str::trim) {
        if line.starts_with('[') {
            clock_section = line.eq_ignore_ascii_case("[clock]");
        }
        if !clock_section || line.starts_with([';', '#']) {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            "language" => settings.language = Language::from_ini(value),
            "time_zone" => settings.clock.time_zone = value.parse().ok(),
            "design" => read_enum(value, &mut settings.clock.design),
            "diameter" => read_number(value, &mut settings.clock.diameter),
            "hands" => read_enum(value, &mut settings.clock.hands),
            "hand_width" => read_number(value, &mut settings.clock.hand_width),
            "numeral_font" => read_enum(value, &mut settings.clock.numeral_font),
            "numeral_size" => read_number(value, &mut settings.clock.numeral_size),
            "numeral_bold" => read_number(value, &mut settings.clock.numeral_bold),
            "numeral_visible" => read_number(value, &mut settings.clock.numeral_visible),
            "snap_to_edges" => read_number(value, &mut settings.clock.snap_to_edges),
            "rim_width" if value.eq_ignore_ascii_case("auto") => settings.clock.rim_width = None,
            "rim_width" => {
                if let Ok(width) = value.parse() {
                    settings.clock.rim_width = Some(width);
                }
            }
            "opacity" => {
                if let Some((opacity, _)) = Opacity::VALUES
                    .into_iter()
                    .find(|(_, label)| label.trim_end_matches('%') == value)
                {
                    settings.opacity = opacity;
                }
            }
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            "always_on_top" => read_number(value, &mut settings.always_on_top),
            "window_x" => x = value.parse().ok(),
            "window_y" => y = value.parse().ok(),
            _ => {}
        }
    }
    settings.clock.normalize();
    settings.position = x.zip(y).map(|(x, y)| pos2(x, y)).filter(|p| p.is_finite());
    settings
}

fn read_enum<T: DeserializeOwned>(input: &str, output: &mut T) {
    if let Ok(value) = ron::from_str(input) {
        *output = value;
    }
}

fn read_number<T: std::str::FromStr>(input: &str, output: &mut T) {
    if let Ok(value) = input.parse() {
        *output = value;
    }
}

fn encode(settings: &Settings) -> String {
    let clock = &settings.clock;
    let mut text = String::from("[clock]\n");
    let mut line = |key: &str, value: &dyn std::fmt::Display| {
        let _ = writeln!(text, "{key}={value}");
    };
    line("language", &settings.language.ini_value());
    line(
        "time_zone",
        &clock.time_zone.map_or("local", |zone| zone.name()),
    );
    line("design", &enum_name(&clock.design));
    line("diameter", &clock.diameter);
    line("hands", &enum_name(&clock.hands));
    line("hand_width", &clock.hand_width);
    line("numeral_font", &enum_name(&clock.numeral_font));
    line("numeral_size", &clock.numeral_size);
    line("numeral_bold", &clock.numeral_bold);
    line("numeral_visible", &clock.numeral_visible);
    line("snap_to_edges", &clock.snap_to_edges);
    line(
        "rim_width",
        &clock
            .rim_width
            .map_or_else(|| "auto".into(), |width| width.to_string()),
    );
    line(
        "opacity",
        &(u16::from(settings.opacity.alpha()) * 100 / 255),
    );
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    line("always_on_top", &settings.always_on_top);
    if let Some(Pos2 { x, y }) = settings.position {
        line("window_x", &x);
        line("window_y", &y);
    }
    text
}

fn enum_name(value: &impl Serialize) -> String {
    ron::to_string(value).expect("settings enum serializes to an INI value")
}
