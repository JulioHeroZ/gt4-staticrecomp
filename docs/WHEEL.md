# Universal Racing Wheel Support (`TASK-018`)

GT4Recomp includes a hardware-agnostic, configurable racing wheel and pedal subsystem (`native/wheel.*`, `gt4l_core::wheel`, `gt4l_platform::sdl_wheels`, and the Launcher **Controls → Racing Wheel** configurator) designed so that **any** PC racing wheel, pedal set, handbrake, or H-shifter works out of the box or with one-click capture.

---

## 1. Supported Hardware & Multi-Device Topologies

Unlike console ports or fixed VID/PID whitelist tables, GT4Recomp enumerates all SDL3 `SDL_Joystick` / DirectInput / USB HID / XInput racing peripherals simultaneously (`wheel:`, `wheel2:`, `wheel3:`, `wheel4:`) and allows combining inputs across independent USB devices:

| Brand / Ecosystem | Supported Bases & Peripherals | Built-in Preset ID |
|---|---|---|
| **Universal / Generic** | Any DirectInput / HID / XInput USB wheel, PXN, Thrustmaster Ferrari, Hori, DIY Arduino/MMOS/OpenFFBoard | `generic` |
| **Logitech** | Driving Force GT (DFGT), Driving Force Pro, Momo, G25, G27, G29, G920, G923, PRO Racing Wheel | `logitech_g29` |
| **Thrustmaster** | T128, T150, TMX, T248, T300 RS, TX, T500 RS, T-GT / T-GT II, TS-PC, TS-XW, T818, T-LCM / T3PA Pedals | `thrustmaster` |
| **Fanatec** | CSL Elite, CSL DD, GT DD Pro, ClubSport Wheel Base V2.5 / DD / DD+, Podium DD1 / DD2, CSL / ClubSport Pedals | `fanatec` |
| **Moza Racing** | R3, R5, R9, R12, R16, R21 Direct Drive bases, SR-P / CRP standalone USB pedals, HGP shifter, HBP handbrake | `moza` |
| **Simagic / Cammus / Simucube** | Simagic Alpha / Mini / Ultimate, M10, Cammus C5 / C12, Simucube 2, Heusinkveld / VNM / Venym load-cell USB pedals | `simagic` |

### Multi-Device Combinations
Every action binding (`steer`, `throttle`, `brake`, `clutch`, `handbrake`, `shift_up`, `shift_down`, `gear_1..gear_7`, `gear_r`, D-Pad, and face buttons) carries its own device slot (`wheel:` for primary wheel, `wheel2:` / `wheel3:` / `wheel4:` for secondary USB devices). When you click **Bind** in the launcher and press a pedal on a standalone USB pedal set or pull a USB handbrake, the launcher automatically records the device index (`wheel2:Axis0 | mode=inverted`) and the native runtime merges all devices seamlessly every frame.

---

## 2. Steering Lock, Deadzone, and Linearity

Configured under `[steering]` in `wheel.toml` or live in the Launcher:

- **`range_deg` (`180°` – `1080°`, default `900°`)**: Effective steering lock. If your hardware base is set to `900°` in its driver and you set `range_deg = 540` in GT4Recomp, reaching `270°` left or right saturates full in-game steering lock (`1.0`).
- **`deadzone` (`0.00` – `0.25`, default `0.01`)**: Center deadzone fraction removed before linearity scaling.
- **`linearity` (`0.25` – `3.00`, default `1.00`)**: Power-curve exponent (`sign(x) * |x|^linearity`). `1.00` is strictly linear (`1:1` proportional to wheel angle); values `> 1.0` soften center sensitivity; values `< 1.0` quicken initial turn-in.
- **`invert` (`true` / `false`)**: Reverses steering axis polarity when needed.

---

## 3. Universal Pedal Axis Modes (`PedalAxisMode`)

Different wheel vendors report pedal axes with wildly different polarities and rest positions. GT4Recomp supports all 5 hardware conventions per pedal (`[pedals]` and per-binding `| mode=...`):

| Mode | Rest → Full Travel | Typical Hardware |
|---|---|---|
| `inverted` | `+32767` (`+1.0`) → `-32768` (`-1.0`) | Logitech G25/G27/G29/G920/G923, Thrustmaster, Fanatec separate pedals |
| `normal` | `-32768` (`-1.0`) → `+32767` (`+1.0`) | Direct Drive bases, Heusinkveld / Moza / Simagic standalone load-cell pedals |
| `positive_half` | `0` (`0.0`) → `+32767` (`+1.0`) | XInput-bridged wheels, combined axis split positive side |
| `negative_half` | `0` (`0.0`) → `-32768` (`-1.0`) | Combined throttle/brake single-axis mode (brake side) |
| `centered` | `0` → `±32767` | Bidirectional analog paddles |

When binding a pedal in the Launcher, **automatic polarity detection** inspects the resting baseline vs. pressed direction and selects `inverted`, `normal`, `positive_half`, or `negative_half` automatically.

---

## 4. Force Feedback & Centering Spring (`[ffb]`)

GT4Recomp drives SDL3 Haptic (`SDL_Haptic`) and Joystick Rumble (`SDL_RumbleJoystick`) feedback:
- **`enabled`**: Master toggle for hardware Force Feedback.
- **`strength` (`0.0` – `2.0`, default `1.0`)**: Overall FFB gain.
- **`spring` (`0.0` – `2.0`, default `0.65`)**: Dynamic speed/angle-proportional centering spring and road-load tension.
- **`damper` (`0.0` – `2.0`, default `0.30`)**: Steering velocity damping.

---

## 5. `wheel.toml` Schema Reference

Stored at `%APPDATA%\GT4Recomp\wheel.toml` and passed to the native executable via `GT4_WHEEL_FILE`:

```toml
schema = 1
enabled = true
preset = "logitech_g29"
device_guid = ""
device_name = ""

[steering]
range_deg = 900
deadzone = 0.010
linearity = 1.000
invert = false

[pedals]
throttle_deadzone = 0.020
throttle_saturation = 0.980
throttle_curve = "linear"
brake_deadzone = 0.020
brake_saturation = 0.950
brake_curve = "linear"
clutch_deadzone = 0.050
clutch_saturation = 0.950
handbrake_deadzone = 0.050
handbrake_saturation = 0.950

[ffb]
enabled = true
strength = 1.000
spring = 0.650
damper = 0.300

[bindings]
steer = ["wheel:Axis0"]
throttle = ["wheel:Axis1 | mode=inverted"]
brake = ["wheel:Axis2 | mode=inverted"]
clutch = ["wheel:Axis3 | mode=inverted"]
handbrake = []
shift_up = ["wheel:Button4"]
shift_down = ["wheel:Button5"]
```
