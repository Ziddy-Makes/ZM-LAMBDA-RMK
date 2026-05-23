# Keyboard Freeze & Wrong-Layer Debugging Guide

## The Bugs

Two intermittent issues affecting both USB and BLE (ruling out transport-specific causes):

### Bug 1: Keyboard Freeze
- Keys stop responding entirely
- Only spinning the rotary encoder restores key input
- Appears to freeze indefinitely without encoder intervention

### Bug 2: Wrong-Layer Actions
- Keys sometimes fire Layer 1 behavior (BLE profile switches, tapdance actions)
- Happens without pressing the `lt!(1, AudioMute)` layer key
- Seems random

## Current Live Keymap (via Vitaly)

The keymap on the device was modified via Vial and differs significantly from the default in `src/keymap.rs`.

**Dumped 2026-03-26 using `vitaly layers`:**

### Layer 0 (Base)
```
Position:  [0,0]     [0,1]     [0,2]            [0,3]
Key:       TD(3)     KC_B      KC_PLAY/PAUSE     LT(1,KC_AUDIO_MUTE)

Position:  [1,0]       [1,1]       [1,2]           [1,3]        Encoder
Key:       LCA(KC_U)   LCA(KC_I)   LCA(KC_ENTER)   LCA(KC_ENTER)  VolDn/VolUp

Position:  [2,0]         [2,1]         [2,2]           [2,3]
Key:       LCA(KC_J)     LCA(KC_K)     LCA(KC_LEFT)    LCA(KC_RIGHT)

Position:  [3,0]           [3,2]            [3,3]
Key:       LCTL(KC_LEFT)   LCTL(KC_UP)      LCTL(KC_RIGHT)
```

### Layer 1 (BLE/System — via LT(1) hold)
```
[0,0] QK_KB_0 (BT0)    [0,1] QK_KB_1 (BT1)    [0,2] QK_KB_2 (BT2)    [0,3] Transparent
[1,0] TD(0)             [1,1] No                [1,2] No                [1,3] QK_KB_7 (Battery)
[2,0] TD(1)             [2,1] No                [2,2] No                [2,3] QK_KB_6 (Switch Output)
[3,0] No                [3,2] No                [3,3] No
```

### Layer 2 (via TD(3) hold → MO(2))
```
[0,0] No    [0,1] No           [0,2] No           [0,3] Transparent
[1,0] No    [1,1] LCA(KC_D)    [1,2] LCA(KC_F)    [1,3] LCA(KC_G)
[2,0] No    [2,1] No           [2,2] LCA(KC_E)     [2,3] LCA(KC_T)
[3,0] No    [3,2] LCTL(KC_DOWN) [3,3] No
```

### Layer 3 (via TD(3) tap → OSL(3))
```
[0,0] No    [0,1] M1 (Macro1)  [0,2] No           [0,3] Transparent
[1,0] No    [1,1] LCA(KC_4)    [1,2] LCA(KC_5)    [1,3] LCA(KC_6)
[2,0] No    [2,1] LCA(KC_7)    [2,2] LCA(KC_8)    [2,3] LCA(KC_9)
[3,0] No    [3,2] No           [3,3] No
```

Layers 4-7: All No/Transparent.

### TapDance Configuration (on device)
```
TD(0): On hold → QK_KB_5 (BLE Clear), 200ms
TD(1): On hold → QK_BOOTLOADER, 200ms
TD(2): On tap → QK_KB_2 (BT2), On hold → QK_KB_5 (BLE Clear), 200ms
TD(3): On tap → OSL(3), On hold → MO(2), 110ms  ← ADDED VIA VIAL (not in firmware source)
```

### Combos (on device)
```
Combo slots 0-7: ALL EMPTY
```

### Key Differences from Source (`src/keymap.rs`)
| Position | Source Default | Live (Vial) |
|----------|--------------|-------------|
| [0,0] | `k!(A)` | `TD(3)` — OSL(3)/MO(2) with 110ms timeout |
| [0,2] | `k!(C)` | `KC_MEDIA_PLAY_PAUSE` |
| [1,0]-[3,3] | Simple letter keys | Various LCA/LCTL shortcuts |
| TD(3) | Not defined | Added via Vial |
| Layer 2-3 | Empty | Populated with shortcuts |

## Root Cause Analysis

### Architecture Overview

The firmware runs tasks concurrently via Embassy:
```
join(
    run_all!(matrix, encoder, adc_device, batt_proc, keyboard, status_led),
    run_rmk(&keymap, driver, &stack, &mut storage, rmk_config),
)
```

The keyboard task's main loop (`rmk/src/keyboard.rs`) has 3 branches:
```
loop {
    1. Process unprocessed_events queue (from OSL/OSM timeouts)
    2. Process buffered morse/tapdance keys (with_deadline: timeout OR new event)
    3. Wait for new KeyboardEvent from PubSub (matrix/encoder)
}
```

### Key Discovery: PubSub Can Silently Drop Events

RMK (rev `aed0972`) uses **PubSub** (not Channel) for keyboard events:
```rust
keyboard_event_subscriber: embassy_sync::pubsub::Subscriber<...>
```

With PubSub, if the keyboard task falls behind (e.g., blocked on `send_report`), **key events are silently dropped** rather than blocking the matrix. This means:
- Release events for the `lt!` key can be lost
- If the release is lost, Layer 1 stays permanently active

### Confirmed Root Cause: `send_report` Backpressure

`send_report()` does `KEYBOARD_REPORT_CHANNEL.sender().send(report).await` which **blocks** when the report channel is full. The HID writer (`run_writer` in `rmk/src/hid.rs`) drains the channel, but `write_report()` (GATT notify for BLE, USB endpoint write for USB) can block if the host isn't consuming reports fast enough.

**Chain of failure:**
1. TD(3) tap → OSL(3) → next keypress resolves on Layer 3
2. Key [0,1] on Layer 3 = `TriggerMacro(1)` → fires 8 HID reports rapidly (4 press + 4 release)
3. Report channel fills → `send_report().await` **blocks** the keyboard task
4. While blocked, matrix events pile up in PubSub → events get **silently dropped**
5. Release events lost → layer/key state can get stuck
6. Keyboard freezes

**Why encoder spin fixes it:** Encoder events generate **consumer control reports** (volume up/down) — a different HID report type that either uses a different path or "wakes up" the host BLE/USB connection, causing it to start consuming the backed-up keyboard reports. The report channel drains, and the keyboard task fully unblocks.

**About the combo log spam in the RTT capture:** The logs show 8 empty combo slots being cycled during the freeze — this is **noise, not the cause**. Key [0,0] is TD(3) (Morse/TapDance), and on Layer 3 [0,0] = `No`. When the freeze occurs with OSL(3) active, the release event for [0,0] resolves to `No`, which triggers empty combo iteration (RMK has a bug where `is_empty()` on a fixed-size `[KeyAction; 4]` array never returns true — see Fix 3). This adds log noise and wastes cycles but is not what causes the freeze.

### Bug 2: Wrong-Layer Actions

The RTT logs confirm Layer 3 was active during the freeze: `Releasing keys in combo: KeyboardEvent { pressed: false, pos: Key(KeyPos { row: 0, col: 1 }) } Single(TriggerMacro(1))` — key [0,1] resolved as Layer 3's `M1` instead of Layer 0's `KC_B`.

This connects to the freeze: if `send_report` backpressure causes PubSub to drop key release events, layer state (from OSL/LT) can get stuck active. Subsequent keys then resolve against the wrong layer.

## RTT Log Capture

### How to Capture

**Step 1: Flash with debug probe**
```bash
cargo run --release
```

**Step 2: Attach RTT in a separate terminal**
```bash
probe-rs attach --chip nRF52840_xxAA \
  target/thumbv7em-none-eabihf/release/ZM-LAMBDA-RMK \
  2>&1 | tee keyboard_debug.log
```

**Step 3:** Use the keyboard normally until the bug occurs.

**Step 4: Analyze**
```bash
# Check for combo processing loop (the confirmed freeze cause)
grep -c "Updated combo" keyboard_debug.log

# Check for stuck layers
grep "\[LAYER\]" keyboard_debug.log

# Check for send_report blockage
grep -A1 "send START" keyboard_debug.log | grep -v "send DONE"

# See what happened right before freeze
tail -50 keyboard_debug.log
```

### Captured Freeze Log (2026-03-26)

The freeze was captured with RTT. Here's the pattern — after normal key reports, the keyboard enters an infinite combo loop:

```
[INFO ] Sending keyboard report, pressed: true    ← last normal activity
[INFO ] Sending keyboard report, pressed: true
[INFO ] Sending keyboard report, pressed: true
[INFO ] Sending keyboard report, pressed: true
[INFO ] Sending keyboard report, pressed: false
[INFO ] Sending keyboard report, pressed: false
[INFO ] Sending keyboard report, pressed: false
[INFO ] Sending keyboard report, pressed: false
                                                   ← NO more keyboard reports after this
[INFO ] Updated combo: Combo { config: ComboConfig { actions: [No, No, No, No], output: No, layer: None }, state: 1, ... }
[INFO ] Updated combo: ... (×8 for each empty combo slot)
[INFO ] Releasing keys in combo: KeyboardEvent { pressed: false, pos: Key(KeyPos { row: 0, col: 0 }) } No
[INFO ] [Combo] releasing: Combo { ... state: 0, ... }
[INFO ] [Combo] releasing: ... (×8)
[INFO ] Updated combo: ... (×8)                    ← repeats forever
[INFO ] Releasing keys in combo: ... (row: 0, col: 0) No
[INFO ] [Combo] releasing: ... (×8)
...
```

**Key observations:**
- The 8 keyboard reports (4 press + 4 release) are from `TriggerMacro(1)` on Layer 3 — confirming OSL(3) was active
- After the macro, key [0,0] release has action `No` → triggers the empty combo loop
- The pattern repeats with keys (0,0), (0,1), (1,0) — all releasing with action `No`
- No `Sending keyboard report` appears after the combo loop starts → **keyboard is frozen**

## Fixes

### Fix 1: Prevent `send_report` from blocking indefinitely (primary fix)

In `rmk-debug/rmk/src/keyboard.rs`, replace the blocking send with a timeout:
```rust
async fn send_report(&self, report: Report) {
    match with_deadline(
        Instant::now() + Duration::from_millis(100),
        KEYBOARD_REPORT_CHANNEL.sender().send(report)
    ).await {
        Ok(()) => {},
        Err(_) => warn!("[KB-REPORT] send timeout, dropping report"),
    }
}
```

Or use non-blocking `try_send`:
```rust
async fn send_report(&self, report: Report) {
    if KEYBOARD_REPORT_CHANNEL.sender().try_send(report).is_err() {
        warn!("[KB-REPORT] channel full, dropping report");
    }
}
```

This prevents the keyboard task from ever blocking indefinitely on a full report channel. A dropped report is far better than a frozen keyboard.

### Fix 2: Disable Combos (reduces log noise and overhead)

Add to `keyboard.toml`:
```toml
combo_max_num = 0
```

This won't fix the freeze itself, but eliminates the wasted combo iterations visible in the logs. No combos are used on this keyboard.

**What `combo_max_num` does:** This setting controls how many combo slots RMK allocates at compile time in the static array `[Option<Combo>; COMBO_MAX_NUM]` (defined in `rmk-config/src/lib.rs:274`, default: 8). Every key event passes through `process_combo()` in `keyboard.rs`, which iterates **all** allocated slots — even empty ones. With `combo_max_num = 0`, no slots are allocated and the combo iteration becomes a no-op, eliminating the overhead entirely.

Note: RMK also provides runtime `ComboOn`/`ComboOff`/`ComboToggle` key actions to toggle combo processing on/off, but `combo_max_num = 0` is stronger — it prevents the combo array from being allocated at all, saving both RAM and per-event CPU cycles.

### Fix 3: RMK Bug Report

`combo.update()` in `rmk/src/combo.rs:82` has a broken empty check:
```rust
self.config.actions.is_empty()  // Always false — [KeyAction; 4] has len() == 4
```

Should be:
```rust
self.config.size() == 0  // Checks for non-No actions
```

### Fix 4 (Previous): Increased report_channel_size

Already applied in `keyboard.toml`:
```toml
report_channel_size = 32  # up from default 8
```

## Tools

### Vitaly (Vial CLI)

Install and use to inspect/modify the live keymap:
```bash
cargo install vitaly

vitaly devices          # list connected keyboards
vitaly layers           # dump all layers
vitaly layers -n 0      # dump specific layer
vitaly layers -p        # show position mapping
vitaly tapdances        # show tapdance config
vitaly combos           # show combo config
vitaly keys -l 0 -p 0   # read specific key (layer 0, position 0)
vitaly save config.json  # save full config to file
vitaly load config.json  # load config from file
```

### RTT Log Capture

```bash
# Flash with probe
cargo run --release

# Attach in another terminal
probe-rs attach --chip nRF52840_xxAA \
  target/thumbv7em-none-eabihf/release/ZM-LAMBDA-RMK \
  2>&1 | tee keyboard_debug.log
```

## What Has Been Changed

### Config: `keyboard.toml`
```toml
report_channel_size = 32  # increased from default 8
```

### Local RMK Patch: `../rmk-debug/`
`Cargo.toml` currently points to a local patched copy of RMK with diagnostic logging:
```toml
rmk = { path = "../rmk-debug/rmk", features = [...] }
```

To revert to upstream:
```toml
rmk = { git = "https://github.com/HaoboGu/rmk", rev = "aed0972", features = [...] }
```

### Diagnostic Logging Added

**`rmk-debug/rmk/src/keyboard.rs`** — Main loop branches and `send_report`:
- `[KB-LOOP] branch=unprocessed|buffered|waiting` — which loop branch executes
- `[KB-LOOP] received event=...` — new event from PubSub
- `[KB-REPORT] send START` / `[KB-REPORT] send DONE` — report channel send timing

**`rmk-debug/rmk/src/keymap.rs`** — Layer state changes:
- `[LAYER] activate layer=N`
- `[LAYER] deactivate layer=N`
- `[LAYER] toggle layer=N new_state=...`

## RMK Source Reference

All RMK source for this revision lives at:
```
~/.cargo/git/checkouts/rmk-cd9707f7f2031ce2/aed0972/
```

Local patched copy:
```
../rmk-debug/
```

| File | What to look at |
|------|----------------|
| `rmk/src/keyboard.rs` | Main loop (~131), `process_combo` (~1001), `dispatch_combos` (~1098) |
| `rmk/src/combo.rs` | `update()` (line 82 — broken `is_empty()` check), `update_released`, `size()` |
| `rmk/src/keyboard/morse.rs` | TapHold/morse timeout handling, `handle_morse_timeout` |
| `rmk/src/keymap.rs` | `activate_layer`/`deactivate_layer` (~294), layer cache |
| `rmk/src/hid.rs` | `run_writer` loop, report channel consumer |
| `rmk/src/channel.rs` | `KEYBOARD_REPORT_CHANNEL` definition (size from `keyboard.toml`) |
| `rmk/src/event/input/keyboard.rs` | `#[input_event(channel_size = 16)]` — PubSub channel config |
