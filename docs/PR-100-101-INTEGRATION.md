# External controller contribution integration

Owner request: 2026-10-25. Accept the contributions one at a time, resolving conflicts and validating
before each merge. This work does not authorize or create another release.

## Baseline and sequence

| Step | Contribution | Integration base | Scope and state |
| --- | --- | --- | --- |
| 1 | PR #100 by luanantony, head `a36486d333c8fa23b89e1bddaa2404979b53dd87` | main `90af154`, stable 1.2.4 plus publication records | TASK-120, dedicated `task/120-dualsense-integration`; static review and native/Rust/UI checks passed; ready for integration |
| 2 | PR #101 by JulioHeroZ, head `921ee810ce453036ee47ae5fcce9fd4190ae6344` at inventory | main after step 1 has been reviewed, validated and merged | Racing-wheel contribution associated with TASK-018; refresh the remote head and map its delta before starting |

The original contributor commits and authorship must remain in the merge ancestry. Use a normal merge
without history rewriting. If the external branch cannot be updated, use a separate integration PR in
our repository and explicitly link its original contribution. Record the exact heads checked before merge.

## Step 1: trigger normalization and curves

The automatic merge conflicts only in TASKS.md. Keep the current TASK-119 release record and the
contribution scope, but rename its incoming TASK-104 entry to TASK-120: TASK-104 already means save
import on the 1.3.0 line. Planned controller tuning TASK-110 remains broader future work.

Review the complete input path, not only the new formulas:

1. SDL3 standardized Gamepad trigger range versus raw joystick/HID ranges; behavior for DualSense,
   standard pads and XInput; identification, explicit override and reconnect semantics.
2. Native pressure conversion, logical binding versus physical trigger selection, deadzones and curves.
3. Rust controls parser/writer, native parser and UI model round-trip compatibility, including old files.
4. Existing RetroAchievements account revision guards, cancelable HTTP and page-response guards remain.

Run focused native mapping/controls fixtures, complete CTest and the Python suite with the exact new
build, then the Rust workspace and real Controls development UI checks. Record failures before fixes
and do not weaken checks. Fresh native outputs and input pins are required when building game candidates.

## Step 2: racing wheels

Start only after step 1 is completed. Re-fetch PR #101 and compare its wheel delta against the newly
merged main. Its history includes the original RetroAchievements contribution; that is already in 1.2.4
with follow-up fixes. Resolve overlap by retaining those fixes rather than restoring old versions.

Expected overlap with step 1 includes SDL input enumeration, platform pad handling, controller service,
controls page/model and native host input. Review combined settings, device ownership and hotplug rules
explicitly. Keep the second integration on its own branch and worktree, with exactly one writer.

Review multi-device axis/button ownership, controller-versus-wheel routing, resting and pressed pedal
polarities, saved per-device identifiers, calibration bounds, disconnected devices and malformed files.
Review force-feedback initialization, strength bounds, reconnect, stop and shutdown behavior. Register
and run native wheel/mapping controls plus the Rust configuration and launcher contract fixtures, using
SDL virtual devices where available. Check that standard gamepads and keyboard mappings still work.

## Evidence limits and protected state

Automated tests can verify formulas, configuration, virtual-device routing and shutdown contracts.
They cannot establish actual DualSense USB/Bluetooth trigger travel or wheel/pedal/force-feedback feel.
Record those as unvalidated hardware checks, never as a passing gameplay result. The owner controls
in-game testing; do not start another game while an active session exists.

Do not change published 1.2.4 or 1.3.0-rc5 assets, version labels or release notes for this integration.
Do not touch installed applications, owner settings, cards, saves or other active worktrees. No private
junctions or symlinks. PCSX2 sources and `native/hw_gs/` remain outside this work.

## Completion record

For each contribution, record the incoming and integration heads, resolved conflict files, substantive
review fixes, exact build/pin locations, test totals and remaining hardware limits in TASKS.md, STATUS
and the daily journal before merge. ai-memory tools were unavailable at preparation; repository records
are the handoff until that service is available again.

## Review corrections prepared for step 1

Confirmed API contract: [SDL_GetGamepadAxis](https://wiki.libsdl.org/SDL3/SDL_GetGamepadAxis) reports
Gamepad triggers as 0 released through 32767 pressed, including DualSense. Raw joystick/HID axes are
not the same API. The incoming automatic signed-range conversion maps SDL zero to pressure 128;
its manual range selector also has no raw input source here. Both are removed, retaining extended
curves and standard Gamepad behavior. No released controls file contained that mode extension.

The production SDL-to-native reading is extracted unchanged into `read_sdl_gamepad`; the host fixture
uses that actual adapter with its own virtual DualSense-identified and Xbox-identified devices. It opens
only those virtual instance IDs and never selects a physical pad or sends haptics. Added rest, quarter,
half, three-quarter and full travel, neutral release and trigger-to-face-button remap assertions.
Rust sampling uses the same byte reduction as native; capture regressions exercise the same standardized
range. New native curve endpoint, monotonicity and exact byte vectors match browser preview checks.
Both parsers reject unsupported `mode` options on triggers and sticks consistently.

The Slint legacy dropdown remains compatible with the new curve enum and its line preview has samples
for each formula. That separate legacy crate is outside the current Cargo workspace and is not claimed
built by workspace validation. Current Tauri production and development paths are the acceptance focus.
Review corrections are prepared but no build or test was executed by this documentation/implementation
writer; the orchestrator must run and record them before merge. No active 1.3.0 worktree is edited or
ported by either integration step.

Step 1 observed checks: CTest 43/43, Python 641 (42 skipped), Rust 381 passed (4 ignored), UI focus 114/114 and navigation 154/154. New immutable slice1200 records the rebuilt game hash; no published asset was changed.
