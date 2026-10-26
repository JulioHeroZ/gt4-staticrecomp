//! Universal racing wheel and multi-USB joystick enumeration, live state reading, smart axis/button capture,
//! and SDL3 Haptic / rumble Force Feedback test trigger (TASK-018).

use gt4l_core::wheel::detect_pedal_axis_mode;

/// Summary of one connected SDL joystick or racing wheel device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WheelDeviceInfo {
    pub id: u32,
    pub guid: String,
    pub name: String,
    pub is_wheel: bool,
    pub is_gamepad: bool,
    pub num_axes: u32,
    pub num_buttons: u32,
    pub num_hats: u32,
    pub has_ffb: bool,
}

/// Live raw state of one connected SDL joystick or racing wheel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawWheelDeviceState {
    pub id: u32,
    pub guid: String,
    pub name: String,
    pub is_wheel: bool,
    pub is_gamepad: bool,
    pub axes: Vec<i16>,
    pub buttons: Vec<bool>,
    pub hats: Vec<u8>,
}

/// Result of capturing a wheel axis, pedal, button, or hat switch in the Launcher UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedWheelInput {
    pub source: String,
    pub detected_pedal_mode: Option<String>,
    pub device_name: String,
}

/// Extracts (Vendor ID, Product ID) from an SDL 32-character hex GUID string.
pub fn parse_guid_vid_pid(guid: &str) -> Option<(u16, u16)> {
    if guid.len() < 20 {
        return None;
    }
    let vid_b0 = u8::from_str_radix(&guid[8..10], 16).ok()?;
    let vid_b1 = u8::from_str_radix(&guid[10..12], 16).ok()?;
    let pid_b0 = u8::from_str_radix(&guid[16..18], 16).ok()?;
    let pid_b1 = u8::from_str_radix(&guid[18..20], 16).ok()?;
    let vid = ((vid_b1 as u16) << 8) | (vid_b0 as u16);
    let pid = ((pid_b1 as u16) << 8) | (pid_b0 as u16);
    Some((vid, pid))
}

pub fn is_wheel_vendor(vid: u16) -> bool {
    matches!(
        vid,
        0x0F0D // HORI
        | 0x046D // Logitech
        | 0x044F // Thrustmaster
        | 0x0EB7 // Fanatec
        | 0x1209 // Moza
        | 0x3416 // Cammus
        | 0x16D0 // SimuCube
    )
}

pub fn format_friendly_device_name(name: &str, guid: &str) -> String {
    if let Some((vid, pid)) = parse_guid_vid_pid(guid) {
        if vid == 0x0F0D {
            if pid == 0x00A4 {
                return format!("HORI Racing Wheel Apex ({name})");
            }
            return format!("HORI Racing Wheel ({name})");
        } else if vid == 0x046D {
            return format!("Logitech Wheel ({name})");
        } else if vid == 0x044F {
            return format!("Thrustmaster Wheel ({name})");
        } else if vid == 0x0EB7 {
            return format!("Fanatec Wheel ({name})");
        }
    }
    name.to_string()
}

/// Returns true if a device is a racing wheel candidate (SDL wheel type, wheel/pedal/shifter keywords,
/// or a non-gamepad USB HID joystick).
pub fn is_candidate_wheel(sdl_is_wheel: bool, sdl_is_gamepad: bool, name: &str, guid: &str) -> bool {
    if sdl_is_wheel {
        return true;
    }
    if let Some((vid, _)) = parse_guid_vid_pid(guid) {
        if is_wheel_vendor(vid) {
            return true;
        }
    }
    let lower = name.to_ascii_lowercase();
    const KEYWORDS: &[&str] = &[
        "wheel",
        "driving force",
        "g29",
        "g920",
        "g923",
        "g27",
        "g25",
        "thrustmaster",
        "t150",
        "t248",
        "t300",
        "t500",
        "t-gt",
        "fanatec",
        "moza",
        "simagic",
        "cammus",
        "simucube",
        "pedal",
        "shifter",
        "handbrake",
        "hori",
        "apex",
        "overdrive",
    ];
    if KEYWORDS.iter().any(|kw| lower.contains(kw)) {
        return true;
    }
    !sdl_is_gamepad
}

/// Resolves which connected device is the active primary wheel for `device_guid`.
pub fn select_primary_wheel_device<'a>(
    device_guid: &str,
    devices: &'a [WheelDeviceInfo],
) -> Option<&'a WheelDeviceInfo> {
    if !device_guid.is_empty() {
        let target = device_guid.to_ascii_lowercase();
        return devices.iter().find(|d| d.guid.to_ascii_lowercase() == target);
    }
    devices
        .iter()
        .find(|d| d.is_wheel)
        .or_else(|| {
            devices.iter().find(|d| {
                parse_guid_vid_pid(&d.guid)
                    .map(|(vid, _)| is_wheel_vendor(vid))
                    .unwrap_or(false)
            })
        })
        .or_else(|| devices.iter().find(|d| !d.is_gamepad))
        .or_else(|| {
            devices.iter().find(|d| {
                parse_guid_vid_pid(&d.guid)
                    .map(|(vid, _)| vid != 0x045E)
                    .unwrap_or(true)
            })
        })
        .or_else(|| devices.first())
}

/// Compares `current` device readings against `baseline` (recorded when the user clicked "Bind")
/// and returns a `CapturedWheelInput` once a clear axis motion, pedal press, button press, or hat press occurs.
pub fn detect_wheel_capture(
    baseline: &[RawWheelDeviceState],
    current: &[RawWheelDeviceState],
    primary_guid: &str,
    target: &str,
) -> Option<CapturedWheelInput> {
    let is_steering = target == "steering";
    let is_pedal = matches!(target, "throttle" | "brake" | "handbrake" | "clutch");

    for cur_dev in current {
        let base_dev = baseline.iter().find(|b| b.id == cur_dev.id)?;
        let prefix = if !primary_guid.is_empty()
            && !cur_dev.guid.eq_ignore_ascii_case(primary_guid)
        {
            format!("dev:{}:", cur_dev.guid.to_ascii_lowercase())
        } else {
            String::new()
        };

        // 1. Axis motion check for steering and pedals:
        if is_steering || is_pedal {
            for (axis_idx, &cur_val) in cur_dev.axes.iter().enumerate() {
                let base_val = base_dev.axes.get(axis_idx).copied().unwrap_or(0);
                let delta = (cur_val as i32 - base_val as i32).abs();
                if delta >= 11000 {
                    let pedal_mode = if is_pedal {
                        Some(detect_pedal_axis_mode(base_val, cur_val).to_string())
                    } else {
                        None
                    };
                    return Some(CapturedWheelInput {
                        source: format!("{prefix}axis:{axis_idx}"),
                        detected_pedal_mode: pedal_mode,
                        device_name: cur_dev.name.clone(),
                    });
                }
            }
            if is_steering {
                continue;
            }
        }

        // 2. Button press check (for button slots or digital pedals):
        for (btn_idx, &cur_pressed) in cur_dev.buttons.iter().enumerate() {
            let was_pressed = base_dev.buttons.get(btn_idx).copied().unwrap_or(false);
            if cur_pressed && !was_pressed {
                return Some(CapturedWheelInput {
                    source: format!("{prefix}btn:{btn_idx}"),
                    detected_pedal_mode: None,
                    device_name: cur_dev.name.clone(),
                });
            }
        }

        // 3. Hat switch check (for button slots):
        if !is_pedal {
            for (hat_idx, &cur_hat) in cur_dev.hats.iter().enumerate() {
                let base_hat = base_dev.hats.get(hat_idx).copied().unwrap_or(0);
                if cur_hat != 0 && cur_hat != base_hat {
                    let dir = if (cur_hat & 0x01) != 0 {
                        Some("up")
                    } else if (cur_hat & 0x02) != 0 {
                        Some("right")
                    } else if (cur_hat & 0x04) != 0 {
                        Some("down")
                    } else if (cur_hat & 0x08) != 0 {
                        Some("left")
                    } else {
                        None
                    };
                    if let Some(dir_name) = dir {
                        return Some(CapturedWheelInput {
                            source: format!("{prefix}hat:{hat_idx}:{dir_name}"),
                            detected_pedal_mode: None,
                            device_name: cur_dev.name.clone(),
                        });
                    }
                }
            }

            // 4. Axis motion check for button slots (analog paddles or triggers):
            for (axis_idx, &cur_val) in cur_dev.axes.iter().enumerate() {
                let base_val = base_dev.axes.get(axis_idx).copied().unwrap_or(0);
                let delta = (cur_val as i32 - base_val as i32).abs();
                if delta >= 18000 {
                    return Some(CapturedWheelInput {
                        source: format!("{prefix}axis:{axis_idx}"),
                        detected_pedal_mode: None,
                        device_name: cur_dev.name.clone(),
                    });
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_detects_inverted_and_normal_pedals_and_multi_device_buttons() {
        let base = vec![
            RawWheelDeviceState {
                id: 1,
                guid: "030000006d0400004fc2000000000000".into(),
                name: "Logitech G29".into(),
                is_wheel: true,
                is_gamepad: false,
                axes: vec![0, 32767, 32767],
                buttons: vec![false; 12],
                hats: vec![0],
            },
            RawWheelDeviceState {
                id: 2,
                guid: "030000004f04000060b6000000000000".into(),
                name: "USB Shifter".into(),
                is_wheel: false,
                is_gamepad: false,
                axes: vec![],
                buttons: vec![false; 8],
                hats: vec![],
            },
        ];

        // Press inverted throttle on axis 1 of primary wheel:
        let mut cur = base.clone();
        cur[0].axes[1] = -20000;
        let cap = detect_wheel_capture(&base, &cur, "030000006d0400004fc2000000000000", "throttle")
            .expect("captured throttle");
        assert_eq!(cap.source, "axis:1");
        assert_eq!(cap.detected_pedal_mode.as_deref(), Some("inverted"));

        // Press button 3 on external USB Shifter:
        let mut cur_btn = base.clone();
        cur_btn[1].buttons[3] = true;
        let cap_btn =
            detect_wheel_capture(&base, &cur_btn, "030000006d0400004fc2000000000000", "r2")
                .expect("captured external shifter button");
        assert_eq!(
            cap_btn.source,
            "dev:030000004f04000060b6000000000000:btn:3"
        );
    }
}
