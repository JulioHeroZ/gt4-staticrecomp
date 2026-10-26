//! The SDL3 gamepad and racing wheel reader. Only the gamepad, joystick and haptic subsystems are started
//! (no video, no window: the Tauri window and its event loop are not touched). Owned by one thread (the launcher's
//! pad service), which calls `pump` and `read` each tick. SDL is not `Send`, so the reader is created on that thread.

use std::collections::{BTreeMap, BTreeSet};

use sdl3::event::Event;
use sdl3::gamepad::{Axis, Button, Gamepad};
use sdl3::joystick::{Joystick, JoystickType};
use sdl3::{EventPump, GamepadSubsystem, HapticSubsystem, JoystickSubsystem, Sdl};

use crate::pad::RawPad;
use crate::sdl_wheels::{format_friendly_device_name, is_candidate_wheel, select_primary_wheel_device, RawWheelDeviceState, WheelDeviceInfo};

/// One connected pad as SDL names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PadInfo {
    /// SDL's joystick instance id (the order "controller 1 of N" uses).
    pub id: u32,
    /// The name SDL reports, e.g. "Xbox 360 Controller" or "DualSense Wireless Controller".
    pub name: String,
    /// SDL's gamepad type string, e.g. "xbox360", "ps5", "switch_pro" (empty when unknown).
    pub kind: String,
    /// SDL's player index, when the platform assigns one.
    pub player_index: Option<u16>,
}

pub struct SdlPads {
    // Kept alive for the subsystems and the event pump; dropping it quits SDL.
    _sdl: Sdl,
    subsystem: GamepadSubsystem,
    joy_subsystem: Option<JoystickSubsystem>,
    haptic_subsystem: Option<HapticSubsystem>,
    events: EventPump,
    open: BTreeMap<u32, Gamepad>,
    open_joysticks: BTreeMap<u32, Joystick>,
}

impl SdlPads {
    /// Starts SDL's gamepad, joystick and haptic subsystems and opens the connected devices.
    pub fn start() -> Result<SdlPads, String> {
        let sdl = sdl3::init().map_err(|error| format!("SDL3 did not start: {error}"))?;
        let subsystem = sdl
            .gamepad()
            .map_err(|error| format!("SDL3 gamepads did not start: {error}"))?;
        let joy_subsystem = sdl.joystick().ok();
        let haptic_subsystem = sdl.haptic().ok();
        let events = sdl
            .event_pump()
            .map_err(|error| format!("SDL3 events did not start: {error}"))?;
        let mut pads = SdlPads {
            _sdl: sdl,
            subsystem,
            joy_subsystem,
            haptic_subsystem,
            events,
            open: BTreeMap::new(),
            open_joysticks: BTreeMap::new(),
        };
        pads.rescan();
        Ok(pads)
    }

    /// Reads SDL's events (hot plug) and refreshes the pad and wheel states. Call once per tick while the launcher is active.
    pub fn pump(&mut self) {
        let mut changed = false;
        for event in self.events.poll_iter() {
            if matches!(
                event,
                Event::GamepadAdded { .. }
                    | Event::GamepadRemoved { .. }
                    | Event::JoyDeviceAdded { .. }
                    | Event::JoyDeviceRemoved { .. }
            ) {
                changed = true;
            }
        }
        if changed {
            self.rescan();
        }
        self.subsystem.update();
        if let Some(joy_sub) = &self.joy_subsystem {
            joy_sub.update();
        }
    }

    /// Opens the pads and joysticks that appeared and closes the ones that went away.
    fn rescan(&mut self) {
        let ids = self.subsystem.gamepads().unwrap_or_default();
        let live: BTreeSet<u32> = ids.iter().map(|id| u32::from(*id)).collect();
        self.open.retain(|id, _| live.contains(id));
        for id in ids {
            let raw = u32::from(id);
            if !self.open.contains_key(&raw) {
                if let Ok(pad) = self.subsystem.open(id) {
                    self.open.insert(raw, pad);
                }
            }
        }

        if let Some(joy_sub) = &self.joy_subsystem {
            let joy_ids = joy_sub.joysticks().unwrap_or_default();
            let live_joy: BTreeSet<u32> = joy_ids.iter().map(|id| u32::from(*id)).collect();
            self.open_joysticks.retain(|id, _| live_joy.contains(id));
            for id in joy_ids {
                let raw = u32::from(id);
                if !self.open_joysticks.contains_key(&raw) {
                    if let Ok(joy) = joy_sub.open(id) {
                        self.open_joysticks.insert(raw, joy);
                    }
                }
            }
        }
    }

    /// The connected pads in SDL's order.
    pub fn connected(&self) -> Vec<PadInfo> {
        self.open
            .iter()
            .map(|(id, pad)| PadInfo {
                id: *id,
                name: pad
                    .name()
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| "Controller".to_string()),
                kind: pad.r#type().string(),
                player_index: pad.player_index(),
            })
            .collect()
    }

    /// The raw reading of one pad, or `None` when it is not connected.
    pub fn read(&self, id: u32) -> Option<RawPad> {
        let pad = self.open.get(&id)?;
        let button = |which: Button| pad.button(which);
        let axis = |which: Axis| pad.axis(which);
        Some(RawPad {
            south: button(Button::South),
            east: button(Button::East),
            west: button(Button::West),
            north: button(Button::North),
            left_shoulder: button(Button::LeftShoulder),
            right_shoulder: button(Button::RightShoulder),
            back: button(Button::Back),
            start: button(Button::Start),
            left_stick: button(Button::LeftStick),
            right_stick: button(Button::RightStick),
            dpad_up: button(Button::DPadUp),
            dpad_down: button(Button::DPadDown),
            dpad_left: button(Button::DPadLeft),
            dpad_right: button(Button::DPadRight),
            left_trigger: axis(Axis::TriggerLeft),
            right_trigger: axis(Axis::TriggerRight),
            left_x: axis(Axis::LeftX),
            left_y: axis(Axis::LeftY),
            right_x: axis(Axis::RightX),
            right_y: axis(Axis::RightY),
        })
    }

    /// Connected joysticks and racing wheels in SDL's order.
    pub fn connected_wheels(&self) -> Vec<WheelDeviceInfo> {
        self.open_joysticks
            .iter()
            .map(|(&id, joy)| {
                let raw_name = joy.name();
                let name = if raw_name.trim().is_empty() {
                    "USB Racing Wheel / Joystick".to_string()
                } else {
                    raw_name
                };
                let guid = joy.guid().to_string().to_ascii_lowercase();
                let sdl_type = unsafe {
                    JoystickType::from_ll(sdl3::sys::joystick::SDL_GetJoystickType(joy.raw()))
                };
                let is_gamepad = self.open.contains_key(&id);
                let is_wheel = is_candidate_wheel(sdl_type == JoystickType::Wheel, is_gamepad, &name, &guid);
                let friendly_name = format_friendly_device_name(&name, &guid);
                let has_ffb = unsafe { sdl3::sys::haptic::SDL_IsJoystickHaptic(joy.raw()) };
                WheelDeviceInfo {
                    id,
                    guid,
                    name: friendly_name,
                    is_wheel,
                    is_gamepad,
                    num_axes: joy.num_axes(),
                    num_buttons: joy.num_buttons(),
                    num_hats: joy.num_hats(),
                    has_ffb,
                }
            })
            .collect()
    }

    /// Live raw state of every connected joystick / racing wheel.
    pub fn read_wheels(&self) -> Vec<RawWheelDeviceState> {
        self.open_joysticks
            .iter()
            .map(|(&id, joy)| {
                let raw_name = joy.name();
                let name = if raw_name.trim().is_empty() {
                    "USB Racing Wheel / Joystick".to_string()
                } else {
                    raw_name
                };
                let guid = joy.guid().to_string().to_ascii_lowercase();
                let sdl_type = unsafe {
                    JoystickType::from_ll(sdl3::sys::joystick::SDL_GetJoystickType(joy.raw()))
                };
                let is_gamepad = self.open.contains_key(&id);
                let is_wheel = is_candidate_wheel(sdl_type == JoystickType::Wheel, is_gamepad, &name, &guid);
                let name = format_friendly_device_name(&name, &guid);

                let num_axes = joy.num_axes();
                let mut axes = Vec::with_capacity(num_axes as usize);
                for a in 0..num_axes {
                    axes.push(joy.axis(a).unwrap_or(0));
                }

                let num_buttons = joy.num_buttons();
                let mut buttons = Vec::with_capacity(num_buttons as usize);
                for b in 0..num_buttons {
                    buttons.push(joy.button(b).unwrap_or(false));
                }

                let num_hats = joy.num_hats();
                let mut hats = Vec::with_capacity(num_hats as usize);
                for h in 0..num_hats {
                    hats.push(joy.hat(h).map(|st| st.to_raw()).unwrap_or(0));
                }

                RawWheelDeviceState {
                    id,
                    guid,
                    name,
                    is_wheel,
                    is_gamepad,
                    axes,
                    buttons,
                    hats,
                }
            })
            .collect()
    }

    /// Plays a short Force Feedback / rumble test pulse on the selected wheel (or primary wheel candidate).
    pub fn test_wheel_ffb(&mut self, device_guid: &str, strength: f64) -> Result<String, String> {
        let infos = self.connected_wheels();
        let target_info = select_primary_wheel_device(device_guid, &infos)
            .or_else(|| infos.first())
            .ok_or_else(|| "No racing wheel or USB controller is connected.".to_string())?;
        let target_id = target_info.id;
        let target_name = target_info.name.clone();

        let joy = self
            .open_joysticks
            .get_mut(&target_id)
            .ok_or_else(|| "Selected wheel is no longer connected.".to_string())?;

        let clamped = (strength.clamp(0.1, 2.0) * 0.65).clamp(0.15, 1.0) as f32;
        let motor_val = (clamped * 65535.0).round() as u16;

        let mut triggered = false;
        if let Some(haptic_sub) = &self.haptic_subsystem {
            if let Ok(mut haptic) = haptic_sub.open_from_joystick_id(target_id) {
                haptic.rumble_play(clamped, 500);
                triggered = true;
            }
        }
        if joy.set_rumble(motor_val, motor_val, 500) {
            triggered = true;
        }

        if triggered {
            Ok(target_name)
        } else {
            Ok(format!("{target_name} (device reported no hardware FFB motor)"))
        }
    }
}
