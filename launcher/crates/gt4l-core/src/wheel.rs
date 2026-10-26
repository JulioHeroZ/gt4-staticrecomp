//! Universal racing wheel configuration (`wheel.toml`, TASK-018).
//!
//! Produces and parses the exact TOML format consumed by `native/wheel.cpp` (`GT4_WHEEL_FILE`),
//! while also serializing cleanly to JSON for the launcher's Wheel settings tab.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Canonical 16 PS2 buttons in the order used by `native/wheel.hpp`.
pub const BUTTON_SLOTS: &[(&str, &str)] = &[
    ("dpad_up", "D-Pad Up"),
    ("dpad_right", "D-Pad Right"),
    ("dpad_down", "D-Pad Down"),
    ("dpad_left", "D-Pad Left"),
    ("cross", "Confirm / Accelerate (Cross)"),
    ("circle", "Handbrake / Cancel (Circle)"),
    ("square", "Brake (Square)"),
    ("triangle", "Reverse / Back (Triangle)"),
    ("l1", "Look Back (L1)"),
    ("r1", "Nitro / High Beam (R1)"),
    ("l2", "Shift Down (L2)"),
    ("r2", "Shift Up (R2)"),
    ("l3", "Horn / L3"),
    ("r3", "R3"),
    ("select", "Camera / View (Select)"),
    ("start", "Pause / Start (Start)"),
];

pub const PRESET_NAMES: &[&str] = &[
    "generic",
    "hori",
    "logitech_g29",
    "logitech_dfgt",
    "thrustmaster",
    "fanatec_moza",
    "combined_pedals",
];

pub const PEDAL_MAP_MODES: &[&str] = &["cross_square", "rstick_y", "l2_r2", "both"];
pub const PEDAL_AXIS_MODES: &[&str] = &[
    "inverted",
    "normal",
    "positive",
    "combined_negative",
    "combined_positive",
];
pub const CURVES: &[&str] = &["linear", "quadratic", "cubic"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SteeringConfig {
    pub source: String,
    pub rotation_degrees: i32,
    pub effective_degrees: i32,
    pub deadzone: f64,
    pub linearity: String,
    pub sensitivity: f64,
    pub invert: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PedalConfig {
    pub source: String,
    pub mode: String,
    pub deadzone: f64,
    pub saturation: f64,
    pub curve: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FfbConfig {
    pub enabled: bool,
    pub strength: f64,
    pub centering_spring: f64,
    pub damper: f64,
    pub curb_rumble: f64,
    pub invert: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WheelConfig {
    pub version: i32,
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub native_dfp: bool,
    pub device_guid: String,
    pub device_name: String,
    pub preset: String,
    pub pedal_map_mode: String,
    pub steering: SteeringConfig,
    pub throttle: PedalConfig,
    pub brake: PedalConfig,
    pub handbrake: PedalConfig,
    pub clutch: PedalConfig,
    pub ffb: FfbConfig,
    pub buttons: BTreeMap<String, Vec<String>>,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self::preset("generic")
    }
}

fn default_buttons_map() -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::new();
    map.insert("dpad_up".into(), vec!["hat:0:up".into()]);
    map.insert("dpad_right".into(), vec!["hat:0:right".into()]);
    map.insert("dpad_down".into(), vec!["hat:0:down".into()]);
    map.insert("dpad_left".into(), vec!["hat:0:left".into()]);
    map.insert("cross".into(), vec!["btn:0".into()]);
    map.insert("square".into(), vec!["btn:1".into()]);
    map.insert("circle".into(), vec!["btn:2".into()]);
    map.insert("triangle".into(), vec!["btn:3".into()]);
    map.insert("r2".into(), vec!["btn:4".into()]);
    map.insert("l2".into(), vec!["btn:5".into()]);
    map.insert("r1".into(), vec!["btn:6".into()]);
    map.insert("l1".into(), vec!["btn:7".into()]);
    map.insert("select".into(), vec!["btn:8".into()]);
    map.insert("start".into(), vec!["btn:9".into()]);
    map.insert("l3".into(), vec!["btn:10".into()]);
    map.insert("r3".into(), vec!["btn:11".into()]);
    map
}

impl WheelConfig {
    pub fn preset(name: &str) -> Self {
        let mut cfg = WheelConfig {
            version: 1,
            enabled: true,
            native_dfp: true,
            device_guid: String::new(),
            device_name: String::new(),
            preset: if name.is_empty() { "generic".into() } else { name.into() },
            pedal_map_mode: "cross_square".into(),
            steering: SteeringConfig {
                source: "axis:0".into(),
                rotation_degrees: 900,
                effective_degrees: 900,
                deadzone: 0.0,
                linearity: "linear".into(),
                sensitivity: 1.0,
                invert: false,
            },
            throttle: PedalConfig {
                source: "axis:1".into(),
                mode: "inverted".into(),
                deadzone: 0.02,
                saturation: 0.98,
                curve: "linear".into(),
            },
            brake: PedalConfig {
                source: "axis:2".into(),
                mode: "inverted".into(),
                deadzone: 0.02,
                saturation: 0.98,
                curve: "linear".into(),
            },
            handbrake: PedalConfig {
                source: String::new(),
                mode: "inverted".into(),
                deadzone: 0.05,
                saturation: 0.95,
                curve: "linear".into(),
            },
            clutch: PedalConfig {
                source: "axis:3".into(),
                mode: "inverted".into(),
                deadzone: 0.05,
                saturation: 0.95,
                curve: "linear".into(),
            },
            ffb: FfbConfig {
                enabled: true,
                strength: 1.0,
                centering_spring: 0.25,
                damper: 0.15,
                curb_rumble: 1.0,
                invert: false,
            },
            buttons: default_buttons_map(),
        };

        match name {
            "generic" | "logitech_g29" | "" => {}
            "hori" => {
                cfg.steering.rotation_degrees = 270;
                cfg.steering.effective_degrees = 270;
                cfg.throttle.source = "axis:5".into();
                cfg.throttle.mode = "normal".into();
                cfg.brake.source = "axis:2".into();
                cfg.brake.mode = "normal".into();
                cfg.clutch.source.clear();
                cfg.handbrake.source.clear();
                cfg.buttons.insert("l2".into(), vec!["btn:4".into()]);
                cfg.buttons.insert("r2".into(), vec!["btn:5".into()]);
                cfg.buttons.insert("cross".into(), vec!["btn:0".into()]);
                cfg.buttons.insert("circle".into(), vec!["btn:1".into()]);
                cfg.buttons.insert("square".into(), vec!["btn:2".into()]);
                cfg.buttons.insert("triangle".into(), vec!["btn:3".into()]);
                cfg.buttons.insert("l1".into(), vec!["btn:8".into()]);
                cfg.buttons.insert("r1".into(), vec!["btn:9".into()]);
                cfg.buttons.insert("select".into(), vec!["btn:6".into()]);
                cfg.buttons.insert("start".into(), vec!["btn:7".into()]);
                cfg.ffb.enabled = false;
            }
            "logitech_dfgt" => {
                cfg.clutch.source.clear();
                cfg.buttons
                    .insert("r2".into(), vec!["btn:4".into(), "btn:12".into()]);
                cfg.buttons
                    .insert("l2".into(), vec!["btn:5".into(), "btn:13".into()]);
            }
            "thrustmaster" => {
                cfg.throttle.source = "axis:2".into();
                cfg.brake.source = "axis:1".into();
                cfg.buttons.insert("l2".into(), vec!["btn:0".into()]);
                cfg.buttons.insert("r2".into(), vec!["btn:1".into()]);
                cfg.buttons.insert("cross".into(), vec!["btn:5".into()]);
                cfg.buttons.insert("square".into(), vec!["btn:3".into()]);
                cfg.buttons.insert("circle".into(), vec!["btn:4".into()]);
                cfg.buttons.insert("triangle".into(), vec!["btn:2".into()]);
            }
            "fanatec_moza" => {
                cfg.throttle.mode = "normal".into();
                cfg.brake.mode = "normal".into();
                cfg.clutch.mode = "normal".into();
                cfg.ffb.centering_spring = 0.15;
                cfg.ffb.damper = 0.10;
            }
            "combined_pedals" => {
                cfg.steering.rotation_degrees = 270;
                cfg.steering.effective_degrees = 270;
                cfg.throttle.source = "axis:1".into();
                cfg.throttle.mode = "combined_negative".into();
                cfg.brake.source = "axis:1".into();
                cfg.brake.mode = "combined_positive".into();
                cfg.clutch.source.clear();
            }
            _ => {
                cfg.preset = "custom".into();
            }
        }
        cfg
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!("Unsupported wheel config version {}", self.version));
        }
        if !self.device_guid.is_empty() && !is_valid_guid(&self.device_guid) {
            return Err("device_guid must be empty or a 32-character hex GUID".into());
        }
        if !PEDAL_MAP_MODES.contains(&self.pedal_map_mode.as_str()) {
            return Err(format!("Invalid pedal_map_mode '{}'", self.pedal_map_mode));
        }
        if !(180..=1080).contains(&self.steering.rotation_degrees) {
            return Err("steering.rotation_degrees must be between 180 and 1080".into());
        }
        if !(180..=1080).contains(&self.steering.effective_degrees) {
            return Err("steering.effective_degrees must be between 180 and 1080".into());
        }
        if !(0.0..=0.45).contains(&self.steering.deadzone) {
            return Err("steering.deadzone must be between 0.0 and 0.45".into());
        }
        if !CURVES.contains(&self.steering.linearity.as_str()) {
            return Err(format!("Invalid steering.linearity '{}'", self.steering.linearity));
        }
        if !(0.25..=2.5).contains(&self.steering.sensitivity) {
            return Err("steering.sensitivity must be between 0.25 and 2.5".into());
        }
        if !self.steering.source.is_empty() {
            validate_source(&self.steering.source, true)?;
        }

        for (label, pedal) in [
            ("throttle", &self.throttle),
            ("brake", &self.brake),
            ("handbrake", &self.handbrake),
            ("clutch", &self.clutch),
        ] {
            if !pedal.source.is_empty() {
                validate_source(&pedal.source, false)?;
            }
            if !PEDAL_AXIS_MODES.contains(&pedal.mode.as_str()) {
                return Err(format!("Invalid {label}.mode '{}'", pedal.mode));
            }
            if !(0.0..=0.45).contains(&pedal.deadzone) {
                return Err(format!("{label}.deadzone must be between 0.0 and 0.45"));
            }
            if !(0.20..=1.0).contains(&pedal.saturation) {
                return Err(format!("{label}.saturation must be between 0.20 and 1.0"));
            }
            if pedal.saturation <= pedal.deadzone + 0.05 {
                return Err(format!("{label}.saturation must exceed deadzone by at least 0.05"));
            }
            if !CURVES.contains(&pedal.curve.as_str()) {
                return Err(format!("Invalid {label}.curve '{}'", pedal.curve));
            }
        }

        if !(0.0..=2.0).contains(&self.ffb.strength) {
            return Err("ffb.strength must be between 0.0 and 2.0".into());
        }
        if !(0.0..=1.0).contains(&self.ffb.centering_spring) {
            return Err("ffb.centering_spring must be between 0.0 and 1.0".into());
        }
        if !(0.0..=1.0).contains(&self.ffb.damper) {
            return Err("ffb.damper must be between 0.0 and 1.0".into());
        }
        if !(0.0..=2.0).contains(&self.ffb.curb_rumble) {
            return Err("ffb.curb_rumble must be between 0.0 and 2.0".into());
        }

        for (slot, list) in &self.buttons {
            if !BUTTON_SLOTS.iter().any(|(name, _)| *name == slot.as_str()) {
                return Err(format!("Unknown button slot '{slot}'"));
            }
            for src in list {
                validate_digital_source(src)?;
            }
        }
        Ok(())
    }

    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        out.push_str("# GT4Recomp racing wheel configuration (TASK-018)\n");
        out.push_str("[wheel]\n");
        out.push_str(&format!("version = {}\n", self.version));
        out.push_str(&format!("enabled = {}\n", self.enabled));
        out.push_str(&format!("native_dfp = {}\n", self.native_dfp));
        out.push_str(&format!("device_guid = \"{}\"\n", sanitize_str(&self.device_guid)));
        out.push_str(&format!("device_name = \"{}\"\n", sanitize_str(&self.device_name)));
        out.push_str(&format!("preset = \"{}\"\n", sanitize_str(&self.preset)));
        out.push_str(&format!(
            "pedal_map_mode = \"{}\"\n\n",
            sanitize_str(&self.pedal_map_mode)
        ));

        out.push_str("[steering]\n");
        out.push_str(&format!("source = \"{}\"\n", sanitize_str(&self.steering.source)));
        out.push_str(&format!("rotation_degrees = {}\n", self.steering.rotation_degrees));
        out.push_str(&format!("effective_degrees = {}\n", self.steering.effective_degrees));
        out.push_str(&format!("deadzone = {:.3}\n", self.steering.deadzone));
        out.push_str(&format!("linearity = \"{}\"\n", sanitize_str(&self.steering.linearity)));
        out.push_str(&format!("sensitivity = {:.3}\n", self.steering.sensitivity));
        out.push_str(&format!("invert = {}\n\n", self.steering.invert));

        for (section, pedal) in [
            ("throttle", &self.throttle),
            ("brake", &self.brake),
            ("handbrake", &self.handbrake),
            ("clutch", &self.clutch),
        ] {
            out.push_str(&format!("[{section}]\n"));
            out.push_str(&format!("source = \"{}\"\n", sanitize_str(&pedal.source)));
            out.push_str(&format!("mode = \"{}\"\n", sanitize_str(&pedal.mode)));
            out.push_str(&format!("deadzone = {:.3}\n", pedal.deadzone));
            out.push_str(&format!("saturation = {:.3}\n", pedal.saturation));
            out.push_str(&format!("curve = \"{}\"\n\n", sanitize_str(&pedal.curve)));
        }

        out.push_str("[ffb]\n");
        out.push_str(&format!("enabled = {}\n", self.ffb.enabled));
        out.push_str(&format!("strength = {:.3}\n", self.ffb.strength));
        out.push_str(&format!("centering_spring = {:.3}\n", self.ffb.centering_spring));
        out.push_str(&format!("damper = {:.3}\n", self.ffb.damper));
        out.push_str(&format!("curb_rumble = {:.3}\n", self.ffb.curb_rumble));
        out.push_str(&format!("invert = {}\n\n", self.ffb.invert));

        out.push_str("[buttons]\n");
        for (slot_name, _) in BUTTON_SLOTS {
            let list = self.buttons.get(*slot_name).cloned().unwrap_or_default();
            let quoted: Vec<String> = list
                .iter()
                .filter(|s| !s.is_empty())
                .map(|s| format!("\"{}\"", sanitize_str(s)))
                .collect();
            out.push_str(&format!("{slot_name} = [{}]\n", quoted.join(", ")));
        }
        out
    }

    pub fn from_toml(text: &str) -> Result<Self, String> {
        let mut cfg = WheelConfig::default();
        let mut section = String::new();
        let mut seen_version = false;

        for (idx, raw_line) in text.lines().enumerate() {
            let line_no = idx + 1;
            let stripped = strip_comment(raw_line, line_no)?;
            let line = stripped.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') {
                if !line.ends_with(']') {
                    return Err(format!("line {line_no}: malformed section header"));
                }
                section = line[1..line.len() - 1].trim().to_string();
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                return Err(format!("line {line_no}: expected key = value"));
            };
            let key = k.trim();
            let val = v.trim();

            match section.as_str() {
                "" | "wheel" => match key {
                    "version" | "schema" => {
                        let ver: i32 = val
                            .parse()
                            .map_err(|_| format!("line {line_no}: invalid version"))?;
                        if ver != 1 {
                            return Err(format!("line {line_no}: unsupported version {ver}"));
                        }
                        cfg.version = 1;
                        seen_version = true;
                    }
                    "enabled" => cfg.enabled = parse_bool(val, line_no)?,
                    "native_dfp" => cfg.native_dfp = parse_bool(val, line_no)?,
                    "device_guid" => cfg.device_guid = unquote(val, line_no)?.to_ascii_lowercase(),
                    "device_name" => cfg.device_name = unquote(val, line_no)?.to_string(),
                    "preset" => cfg.preset = unquote(val, line_no)?.to_string(),
                    "pedal_map_mode" => cfg.pedal_map_mode = unquote(val, line_no)?.to_string(),
                    other => return Err(format!("line {line_no}: unknown [wheel] key '{other}'")),
                },
                "steering" => match key {
                    "source" => cfg.steering.source = unquote(val, line_no)?.to_string(),
                    "rotation_degrees" => {
                        cfg.steering.rotation_degrees = val
                            .parse()
                            .map_err(|_| format!("line {line_no}: invalid rotation_degrees"))?
                    }
                    "effective_degrees" => {
                        cfg.steering.effective_degrees = val
                            .parse()
                            .map_err(|_| format!("line {line_no}: invalid effective_degrees"))?
                    }
                    "deadzone" => {
                        cfg.steering.deadzone = val
                            .parse()
                            .map_err(|_| format!("line {line_no}: invalid deadzone"))?
                    }
                    "linearity" => cfg.steering.linearity = unquote(val, line_no)?.to_string(),
                    "sensitivity" => {
                        cfg.steering.sensitivity = val
                            .parse()
                            .map_err(|_| format!("line {line_no}: invalid sensitivity"))?
                    }
                    "invert" => cfg.steering.invert = parse_bool(val, line_no)?,
                    other => return Err(format!("line {line_no}: unknown [steering] key '{other}'")),
                },
                "throttle" => apply_pedal_kv(&mut cfg.throttle, key, val, line_no)?,
                "brake" => apply_pedal_kv(&mut cfg.brake, key, val, line_no)?,
                "handbrake" => apply_pedal_kv(&mut cfg.handbrake, key, val, line_no)?,
                "clutch" => apply_pedal_kv(&mut cfg.clutch, key, val, line_no)?,
                "ffb" => match key {
                    "enabled" => cfg.ffb.enabled = parse_bool(val, line_no)?,
                    "strength" => {
                        cfg.ffb.strength = val
                            .parse()
                            .map_err(|_| format!("line {line_no}: invalid ffb.strength"))?
                    }
                    "centering_spring" => {
                        cfg.ffb.centering_spring = val
                            .parse()
                            .map_err(|_| format!("line {line_no}: invalid ffb.centering_spring"))?
                    }
                    "damper" => {
                        cfg.ffb.damper = val
                            .parse()
                            .map_err(|_| format!("line {line_no}: invalid ffb.damper"))?
                    }
                    "curb_rumble" => {
                        cfg.ffb.curb_rumble = val
                            .parse()
                            .map_err(|_| format!("line {line_no}: invalid ffb.curb_rumble"))?
                    }
                    "invert" => cfg.ffb.invert = parse_bool(val, line_no)?,
                    other => return Err(format!("line {line_no}: unknown [ffb] key '{other}'")),
                },
                "buttons" => {
                    let canonical = resolve_button_key(key)
                        .ok_or_else(|| format!("line {line_no}: unknown button '{key}'"))?;
                    let items = parse_array(val, line_no)?;
                    cfg.buttons.insert(canonical.to_string(), items);
                }
                other => return Err(format!("line {line_no}: unknown section '[{other}]'")),
            }
        }

        if !seen_version {
            return Err("missing version = 1 in wheel.toml".into());
        }
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn load_or_default(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Self::from_toml(&text)
    }

    pub fn save_atomic(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        let tmp = path.with_extension("toml.tmp");
        let content = self.to_toml();
        fs::write(&tmp, content.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        fs::rename(&tmp, path).map_err(|e| format!("cannot replace {}: {e}", path.display()))
    }

    /// Updates a single setting key (e.g. `"wheel.enabled"`, `"steering.rotation_degrees"`, `"throttle.mode"`, `"ffb.strength"`).
    pub fn set_setting(&mut self, key: &str, value: &serde_json::Value) -> Result<(), String> {
        match key {
            "enabled" | "wheel.enabled" => {
                self.enabled = value.as_bool().ok_or("expected boolean for wheel.enabled")?;
            }
            "native_dfp" | "wheel.native_dfp" => {
                self.native_dfp = value.as_bool().ok_or("expected boolean for wheel.native_dfp")?;
            }
            "device_guid" | "wheel.device_guid" => {
                self.device_guid = value
                    .as_str()
                    .ok_or("expected string for wheel.device_guid")?
                    .trim()
                    .to_ascii_lowercase();
            }
            "wheel.device_name" => {
                self.device_name = value
                    .as_str()
                    .ok_or("expected string for wheel.device_name")?
                    .to_string();
            }
            "wheel.pedal_map_mode" => {
                self.pedal_map_mode = value
                    .as_str()
                    .ok_or("expected string for wheel.pedal_map_mode")?
                    .to_string();
            }
            "steering.source" => {
                self.steering.source = value
                    .as_str()
                    .ok_or("expected string for steering.source")?
                    .to_string();
                self.preset = "custom".into();
            }
            "steering.rotation_degrees" => {
                let deg = value
                    .as_i64()
                    .ok_or("expected integer for steering.rotation_degrees")? as i32;
                self.steering.rotation_degrees = deg;
                if self.steering.effective_degrees > deg {
                    self.steering.effective_degrees = deg;
                }
            }
            "steering.effective_degrees" => {
                let deg = value
                    .as_i64()
                    .ok_or("expected integer for steering.effective_degrees")? as i32;
                self.steering.effective_degrees = deg;
                if self.steering.rotation_degrees < deg {
                    self.steering.rotation_degrees = deg;
                }
            }
            "steering.deadzone" => {
                self.steering.deadzone = value
                    .as_f64()
                    .ok_or("expected number for steering.deadzone")?;
            }
            "steering.linearity" => {
                self.steering.linearity = value
                    .as_str()
                    .ok_or("expected string for steering.linearity")?
                    .to_string();
            }
            "steering.sensitivity" => {
                self.steering.sensitivity = value
                    .as_f64()
                    .ok_or("expected number for steering.sensitivity")?;
            }
            "steering.invert" => {
                self.steering.invert = value
                    .as_bool()
                    .ok_or("expected boolean for steering.invert")?;
            }
            "ffb.enabled" => {
                self.ffb.enabled = value.as_bool().ok_or("expected boolean for ffb.enabled")?;
            }
            "ffb.strength" => {
                self.ffb.strength = value.as_f64().ok_or("expected number for ffb.strength")?;
            }
            "ffb.centering_spring" => {
                self.ffb.centering_spring = value
                    .as_f64()
                    .ok_or("expected number for ffb.centering_spring")?;
            }
            "ffb.damper" => {
                self.ffb.damper = value.as_f64().ok_or("expected number for ffb.damper")?;
            }
            "ffb.curb_rumble" => {
                self.ffb.curb_rumble = value
                    .as_f64()
                    .ok_or("expected number for ffb.curb_rumble")?;
            }
            "ffb.invert" => {
                self.ffb.invert = value.as_bool().ok_or("expected boolean for ffb.invert")?;
            }
            _ => {
                let Some((section, field)) = key.split_once('.') else {
                    return Err(format!("Unknown wheel setting '{key}'"));
                };
                let pedal = match section {
                    "throttle" => &mut self.throttle,
                    "brake" => &mut self.brake,
                    "handbrake" => &mut self.handbrake,
                    "clutch" => &mut self.clutch,
                    _ => return Err(format!("Unknown wheel setting '{key}'")),
                };
                match field {
                    "source" => {
                        pedal.source = value
                            .as_str()
                            .ok_or("expected string for pedal source")?
                            .to_string();
                        self.preset = "custom".into();
                    }
                    "mode" => {
                        pedal.mode = value
                            .as_str()
                            .ok_or("expected string for pedal mode")?
                            .to_string();
                    }
                    "deadzone" => {
                        let dz = value.as_f64().ok_or("expected number for pedal deadzone")?;
                        pedal.deadzone = dz;
                        if pedal.saturation <= dz + 0.05 {
                            pedal.saturation = (dz + 0.06).clamp(0.20, 1.0);
                        }
                    }
                    "saturation" => {
                        let sat = value
                            .as_f64()
                            .ok_or("expected number for pedal saturation")?;
                        pedal.saturation = sat;
                        if pedal.saturation <= pedal.deadzone + 0.05 {
                            pedal.deadzone = (sat - 0.06).clamp(0.0, 0.45);
                        }
                    }
                    "curve" => {
                        pedal.curve = value
                            .as_str()
                            .ok_or("expected string for pedal curve")?
                            .to_string();
                    }
                    _ => return Err(format!("Unknown pedal field '{key}'")),
                }
            }
        }
        self.validate()
    }

    /// Assigns a captured or selected binding to `"steering"`, `"throttle"`, `"brake"`, `"handbrake"`, `"clutch"`,
    /// or any of the 16 button slots.
    pub fn assign_binding(
        &mut self,
        target: &str,
        source: &str,
        detected_pedal_mode: Option<&str>,
    ) -> Result<(), String> {
        let source = source.trim();
        match target {
            "steering" => {
                if !source.is_empty() {
                    validate_source(source, true)?;
                }
                self.steering.source = source.to_string();
            }
            "throttle" | "brake" | "handbrake" | "clutch" => {
                if !source.is_empty() {
                    validate_source(source, false)?;
                }
                let pedal = match target {
                    "throttle" => &mut self.throttle,
                    "brake" => &mut self.brake,
                    "handbrake" => &mut self.handbrake,
                    _ => &mut self.clutch,
                };
                pedal.source = source.to_string();
                if let Some(mode) = detected_pedal_mode {
                    if PEDAL_AXIS_MODES.contains(&mode) {
                        pedal.mode = mode.to_string();
                    }
                }
            }
            _ => {
                let Some(slot) = resolve_button_key(target) else {
                    return Err(format!("Unknown binding target '{target}'"));
                };
                if source.is_empty() {
                    self.buttons.insert(slot.to_string(), Vec::new());
                } else {
                    validate_digital_source(source)?;
                    let entry = self.buttons.entry(slot.to_string()).or_default();
                    if !entry.iter().any(|s| s == source) {
                        if entry.len() >= 3 {
                            entry.remove(0);
                        }
                        entry.push(source.to_string());
                    }
                }
            }
        }
        self.preset = "custom".into();
        self.validate()
    }

    /// Resets one target (`"steering"`, `"throttle"`, `"brake"`, `"handbrake"`, `"clutch"`, or a button slot)
    /// to its default binding (or unbinds it when `clear` is true).
    pub fn reset_binding(&mut self, target: &str, clear: bool) -> Result<(), String> {
        let defaults = Self::preset("generic");
        match target {
            "steering" => {
                self.steering.source = if clear {
                    String::new()
                } else {
                    defaults.steering.source
                };
            }
            "throttle" => {
                self.throttle.source = if clear {
                    String::new()
                } else {
                    defaults.throttle.source
                };
            }
            "brake" => {
                self.brake.source = if clear {
                    String::new()
                } else {
                    defaults.brake.source
                };
            }
            "handbrake" => {
                self.handbrake.source = if clear {
                    String::new()
                } else {
                    defaults.handbrake.source
                };
            }
            "clutch" => {
                self.clutch.source = if clear {
                    String::new()
                } else {
                    defaults.clutch.source
                };
            }
            _ => {
                let Some(slot) = resolve_button_key(target) else {
                    return Err(format!("Unknown binding target '{target}'"));
                };
                let replacement = if clear {
                    Vec::new()
                } else {
                    defaults.buttons.get(slot).cloned().unwrap_or_default()
                };
                self.buttons.insert(slot.to_string(), replacement);
            }
        }
        self.validate()
    }
}

/// Evaluates calibrated steering in `-1.0..=1.0` from a raw 16-bit signed axis value.
pub fn evaluate_steering_normalized(cfg: &SteeringConfig, raw: i16) -> f64 {
    let mut signed = if raw < 0 {
        (raw as f64) / 32768.0
    } else {
        (raw as f64) / 32767.0
    };
    if cfg.invert {
        signed = -signed;
    }
    let sign = if signed < 0.0 { -1.0 } else { 1.0 };
    let mut mag = signed.abs();
    let phys = cfg.rotation_degrees.clamp(180, 1080) as f64;
    let eff = cfg.effective_degrees.clamp(180, cfg.rotation_degrees.clamp(180, 1080)) as f64;
    let ratio = eff / phys;
    if ratio > 0.0 {
        mag /= ratio;
    }
    let dz = cfg.deadzone.clamp(0.0, 0.45);
    if mag <= dz {
        return 0.0;
    }
    let norm = ((mag - dz) / (1.0 - dz)).clamp(0.0, 1.0);
    let curved = apply_curve(&cfg.linearity, norm);
    let scaled = (curved * cfg.sensitivity.clamp(0.25, 2.5)).clamp(0.0, 1.0);
    sign * scaled
}

/// Evaluates calibrated pedal travel in `0.0..=1.0` from a raw 16-bit signed axis value.
pub fn evaluate_pedal_normalized(cfg: &PedalConfig, raw: i16) -> f64 {
    let travel = match cfg.mode.as_str() {
        "inverted" => ((32767.0 - (raw as f64)) / 65535.0).clamp(0.0, 1.0),
        "normal" => (((raw as f64) + 32768.0) / 65535.0).clamp(0.0, 1.0),
        "positive" | "combined_positive" => {
            if raw <= 0 {
                0.0
            } else {
                ((raw as f64) / 32767.0).clamp(0.0, 1.0)
            }
        }
        "combined_negative" => {
            if raw >= 0 {
                0.0
            } else {
                ((-(raw as i32) as f64) / 32768.0).clamp(0.0, 1.0)
            }
        }
        _ => 0.0,
    };
    let dz = cfg.deadzone.clamp(0.0, 0.45);
    let sat = cfg.saturation.clamp((dz + 0.05).min(1.0), 1.0);
    if travel <= dz {
        return 0.0;
    }
    let norm = ((travel - dz) / (sat - dz)).clamp(0.0, 1.0);
    apply_curve(&cfg.curve, norm).clamp(0.0, 1.0)
}

/// Auto-detects the pedal axis polarity mode given its resting raw value and pressed raw value.
/// Auto-detects the matching wheel preset based on connected device name and axis signature.
pub fn detect_wheel_preset(device_name: &str, num_axes: usize, axes: Option<&[i16]>) -> &'static str {
    let lower = device_name.to_ascii_lowercase();
    if lower.contains("hori") || lower.contains("apex") || lower.contains("overdrive") {
        return "hori";
    }
    if lower.contains("driving force") || lower.contains("dfgt") {
        return "logitech_dfgt";
    }
    if lower.contains("g29")
        || lower.contains("g920")
        || lower.contains("g923")
        || lower.contains("g27")
        || lower.contains("g25")
        || lower.contains("logitech")
    {
        return "logitech_g29";
    }
    if lower.contains("thrustmaster")
        || lower.contains("t150")
        || lower.contains("t248")
        || lower.contains("t300")
        || lower.contains("t-gt")
    {
        return "thrustmaster";
    }
    if lower.contains("fanatec")
        || lower.contains("csl dd")
        || lower.contains("gt dd")
        || lower.contains("moza")
        || lower.contains("simagic")
    {
        return "fanatec_moza";
    }
    if num_axes >= 6 && (lower.contains("xbox") || lower.contains("xinput")) {
        if let Some(vals) = axes {
            if vals.len() >= 6 && vals[2] < -16000 && vals[5] < -16000 {
                return "hori";
            }
        }
        return "hori";
    }
    "generic"
}

pub fn detect_pedal_axis_mode(rest_raw: i16, pressed_raw: i16) -> &'static str {
    if rest_raw > 16000 && pressed_raw < rest_raw - 8000 {
        "inverted"
    } else if rest_raw < -16000 && pressed_raw > rest_raw + 8000 {
        "normal"
    } else if rest_raw.abs() <= 16000 && pressed_raw < -10000 {
        "combined_negative"
    } else if rest_raw.abs() <= 16000 && pressed_raw > 10000 {
        "positive"
    } else if pressed_raw < rest_raw {
        "inverted"
    } else {
        "normal"
    }
}

fn apply_curve(curve: &str, v: f64) -> f64 {
    match curve {
        "quadratic" => v * v,
        "cubic" => v * v * v,
        _ => v,
    }
}

fn sanitize_str(s: &str) -> String {
    s.chars().filter(|c| *c != '"' && *c != '\n' && *c != '\r').collect()
}

fn is_valid_guid(guid: &str) -> bool {
    guid.len() == 32 && guid.chars().all(|c| c.is_ascii_hexdigit())
}

fn strip_comment(line: &str, line_no: usize) -> Result<&str, String> {
    let mut in_quotes = false;
    let bytes = line.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'"' {
            if i > 0 && bytes[i - 1] == b'\\' {
                return Err(format!("line {line_no}: escape sequences are not supported"));
            }
            in_quotes = !in_quotes;
        } else if bytes[i] == b'#' && !in_quotes {
            return Ok(&line[..i]);
        }
    }
    if in_quotes {
        return Err(format!("line {line_no}: unterminated string"));
    }
    Ok(line)
}

fn unquote(token: &str, line_no: usize) -> Result<&str, String> {
    let t = token.trim();
    if t.len() < 2 || !t.starts_with('"') || !t.ends_with('"') {
        return Err(format!("line {line_no}: expected double-quoted string"));
    }
    let inner = &t[1..t.len() - 1];
    if inner.contains('"') {
        return Err(format!("line {line_no}: unexpected quote inside string"));
    }
    Ok(inner)
}

fn parse_bool(token: &str, line_no: usize) -> Result<bool, String> {
    match token.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("line {line_no}: expected true or false")),
    }
}

fn parse_array(raw: &str, line_no: usize) -> Result<Vec<String>, String> {
    let t = raw.trim();
    if t.starts_with('"') {
        let s = unquote(t, line_no)?;
        return Ok(if s.is_empty() { Vec::new() } else { vec![s.to_string()] });
    }
    if t.len() < 2 || !t.starts_with('[') || !t.ends_with(']') {
        return Err(format!("line {line_no}: expected [...] array"));
    }
    let body = t[1..t.len() - 1].trim();
    if body.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for part in body.split(',') {
        let s = unquote(part.trim(), line_no)?;
        if !s.is_empty() && !out.iter().any(|existing| existing == s) {
            out.push(s.to_string());
        }
    }
    Ok(out)
}

fn apply_pedal_kv(pedal: &mut PedalConfig, key: &str, val: &str, line_no: usize) -> Result<(), String> {
    match key {
        "source" => pedal.source = unquote(val, line_no)?.to_string(),
        "mode" => pedal.mode = unquote(val, line_no)?.to_string(),
        "deadzone" => {
            pedal.deadzone = val
                .parse()
                .map_err(|_| format!("line {line_no}: invalid pedal deadzone"))?
        }
        "saturation" => {
            pedal.saturation = val
                .parse()
                .map_err(|_| format!("line {line_no}: invalid pedal saturation"))?
        }
        "curve" => pedal.curve = unquote(val, line_no)?.to_string(),
        other => return Err(format!("line {line_no}: unknown pedal key '{other}'")),
    }
    Ok(())
}

pub fn resolve_button_key(key: &str) -> Option<&'static str> {
    for (name, _) in BUTTON_SLOTS {
        if *name == key {
            return Some(*name);
        }
    }
    match key {
        "shift_up" | "gear_up" | "paddle_right" => Some("r2"),
        "shift_down" | "gear_down" | "paddle_left" => Some("l2"),
        "look_back" | "rear_view" => Some("l1"),
        "nitro" | "boost" | "high_beam" => Some("r1"),
        "reverse" => Some("triangle"),
        "handbrake_btn" | "ebrake" => Some("circle"),
        "camera" | "view" | "change_view" => Some("select"),
        "pause" => Some("start"),
        "horn" => Some("l3"),
        _ => None,
    }
}

fn strip_dev_prefix(src: &str) -> Result<&str, String> {
    if let Some(rest) = src.strip_prefix("dev:") {
        let Some((guid, tail)) = rest.split_once(':') else {
            return Err(format!("Malformed device source '{src}'"));
        };
        if !is_valid_guid(guid) {
            return Err(format!("Invalid device GUID '{guid}' in '{src}'"));
        }
        Ok(tail)
    } else {
        Ok(src)
    }
}

fn validate_source(src: &str, axis_only: bool) -> Result<(), String> {
    let body = strip_dev_prefix(src)?;
    if let Some(idx_str) = body.strip_prefix("axis:") {
        let idx: u8 = idx_str
            .parse()
            .map_err(|_| format!("Invalid axis index in '{src}'"))?;
        if idx > 31 {
            return Err(format!("Axis index {idx} out of range (0..31)"));
        }
        return Ok(());
    }
    if axis_only {
        return Err(format!("Steering source '{src}' must be an axis"));
    }
    validate_digital_source(src)
}

fn validate_digital_source(src: &str) -> Result<(), String> {
    let body = strip_dev_prefix(src)?;
    if let Some(idx_str) = body.strip_prefix("btn:") {
        let idx: u8 = idx_str
            .parse()
            .map_err(|_| format!("Invalid button index in '{src}'"))?;
        if idx > 127 {
            return Err(format!("Button index {idx} out of range (0..127)"));
        }
        return Ok(());
    }
    if let Some(rest) = body.strip_prefix("hat:") {
        let Some((hat_idx_str, dir)) = rest.split_once(':') else {
            return Err(format!("Malformed hat source '{src}'"));
        };
        let idx: u8 = hat_idx_str
            .parse()
            .map_err(|_| format!("Invalid hat index in '{src}'"))?;
        if idx > 7 {
            return Err(format!("Hat index {idx} out of range (0..7)"));
        }
        if !matches!(dir, "up" | "right" | "down" | "left") {
            return Err(format!("Invalid hat direction '{dir}' in '{src}'"));
        }
        return Ok(());
    }
    if let Some(rest) = body.strip_prefix("axis:") {
        let clean = rest
            .strip_suffix('+')
            .or_else(|| rest.strip_suffix('-'))
            .unwrap_or(rest);
        let idx: u8 = clean
            .parse()
            .map_err(|_| format!("Invalid axis index in '{src}'"))?;
        if idx > 31 {
            return Err(format!("Axis index {idx} out of range (0..31)"));
        }
        return Ok(());
    }
    Err(format!("Unknown digital wheel source '{src}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_roundtrip_toml_and_validate() {
        for preset in PRESET_NAMES {
            let cfg = WheelConfig::preset(preset);
            assert!(cfg.validate().is_ok(), "preset {preset} valid");
            let toml = cfg.to_toml();
            let parsed = WheelConfig::from_toml(&toml).expect("roundtrip parse");
            assert_eq!(parsed, cfg);
        }
    }

    #[test]
    fn steering_and_pedal_evaluation_and_polarity_detection() {
        let mut cfg = WheelConfig::default();
        assert_eq!(evaluate_steering_normalized(&cfg.steering, 0), 0.0);
        assert!((evaluate_steering_normalized(&cfg.steering, 32767) - 1.0).abs() < 1e-6);
        assert!((evaluate_steering_normalized(&cfg.steering, -32768) + 1.0).abs() < 1e-6);

        cfg.steering.effective_degrees = 540;
        assert!((evaluate_steering_normalized(&cfg.steering, (32767 * 60 / 100 + 1) as i16) - 1.0).abs() < 1e-6);

        assert_eq!(evaluate_pedal_normalized(&cfg.throttle, 32767), 0.0);
        assert!((evaluate_pedal_normalized(&cfg.throttle, -32768) - 1.0).abs() < 1e-6);

        assert_eq!(detect_pedal_axis_mode(32767, -30000), "inverted");
        assert_eq!(detect_pedal_axis_mode(-32768, 30000), "normal");
        assert_eq!(detect_pedal_axis_mode(0, -30000), "combined_negative");
        assert_eq!(detect_pedal_axis_mode(0, 30000), "positive");
    }

    #[test]
    fn set_setting_and_assign_binding_update_and_validate() {
        let mut cfg = WheelConfig::default();
        cfg.set_setting("steering.rotation_degrees", &serde_json::json!(540))
            .unwrap();
        assert_eq!(cfg.steering.rotation_degrees, 540);
        assert_eq!(cfg.steering.effective_degrees, 540);

        cfg.assign_binding(
            "throttle",
            "dev:030000006d0400004fc2000000000000:axis:2",
            Some("normal"),
        )
        .unwrap();
        assert_eq!(
            cfg.throttle.source,
            "dev:030000006d0400004fc2000000000000:axis:2"
        );
        assert_eq!(cfg.throttle.mode, "normal");

        cfg.assign_binding("shift_up", "btn:14", None).unwrap();
        assert!(cfg.buttons["r2"].contains(&"btn:14".to_string()));
    }
}
