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

The `lt!(1, AudioMute)` key is a `TapHold` — RMK treats it as a "morse" key internally. When pressed, it enters the held buffer with a 250ms timeout to decide tap vs hold.

### Key Discovery: PubSub Can Silently Drop Events

RMK (rev `aed0972`) uses **PubSub** (not Channel) for keyboard events:
```rust
keyboard_event_subscriber: embassy_sync::pubsub::Subscriber<...>
```

With PubSub, if the keyboard task falls behind (e.g., blocked on `send_report`), **key events are silently dropped** rather than blocking the matrix. This means:
- Release events for the `lt!` key can be lost
- If the release is lost, Layer 1 stays permanently active
- This directly explains Bug 2

### Hypothesis for Bug 1 (Freeze): `send_report` Backpressure

`send_report()` does `KEYBOARD_REPORT_CHANNEL.sender().send(report).await` which **blocks** when the report channel is full (default size: 8).

The HID writer (`run_writer` in `rmk/src/hid.rs`) drains the report channel, but `write_report()` (GATT notify for BLE, USB endpoint write for USB) can block if the host isn't consuming reports fast enough.

**Chain of failure:**
1. HID writer blocks on `write_report()` (host not consuming)
2. Report channel fills up (8 slots)
3. Keyboard task blocks on `send_report().await`
4. Can't process any more key events
5. PubSub events from matrix pile up and get dropped
6. Everything appears frozen

**Why encoder spin fixes it:** The encoder generates events on the same PubSub. When the report channel eventually drains (even partially), the keyboard task unblocks, processes the encoder event (media key via a different report path), and recovers.

### Alternative Freeze Hypothesis: Buffered Key Loop

The keyboard task could enter a tight loop where `next_buffered_key()` keeps returning a stale buffered key whose timeout fires immediately (timestamp bug). The `handle_morse_timeout` runs repeatedly without yielding. An encoder event received in `with_deadline` breaks this loop.

### Hypothesis for Bug 2 (Wrong Layer): Lost Release Event

When holding `lt!(1, AudioMute)`:
1. Hold timeout fires -> `LayerOn(1)` processed -> **Layer 1 activated**
2. Key state -> `ProcessedButReleaseNotReportedYet(LayerOn(1))`
3. On release -> `LayerOn(1)` with release event -> **Layer 1 deactivated**

If step 3 fails (release event dropped by PubSub during a freeze, or held buffer corrupted), Layer 1 stays active permanently. All subsequent key presses resolve against Layer 1 mappings.

The two bugs are likely connected: a freeze while Layer 1 is active causes the release event to be dropped, leaving Layer 1 stuck.

## What Has Been Changed

### Config: `keyboard.toml`
```toml
report_channel_size = 32  # increased from default 8
```
Gives more buffer before backpressure blocks the keyboard task.

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

## How to Capture RTT Logs

### Step 1: Flash with debug probe
```bash
cargo run --release
```

### Step 2: Attach RTT in a separate terminal
```bash
probe-rs attach --chip nRF52840_xxAA \
  target/thumbv7em-none-eabihf/release/ZM-LAMBDA-RMK \
  2>&1 | tee keyboard_debug.log
```

### Step 3: Use the keyboard normally until the bug occurs

### Step 4: Analyze the log file
```bash
# Check for send_report that never completed (freeze cause)
grep -A1 "send START" keyboard_debug.log | grep -v "send DONE"

# Check for stuck layers (activate without matching deactivate)
grep "\[LAYER\]" keyboard_debug.log

# Check which loop branch was last entered before freeze
tail -50 keyboard_debug.log | grep "\[KB-LOOP\]"
```

## What to Look For in Logs

### Confirming Bug 1 (Freeze) — `send_report` backpressure
```
[KB-REPORT] send START     <-- appears
                            <-- NO "send DONE" follows
                            <-- keyboard is blocked here
```
If you see `send START` as the last report-related log before the freeze, the keyboard is stuck waiting for the report channel to drain.

### Confirming Bug 1 (Freeze) — Buffered key tight loop
```
[KB-LOOP] branch=buffered event=... state=Pressed(...)
[KB-LOOP] branch=buffered event=... state=Pressed(...)
[KB-LOOP] branch=buffered event=... state=Pressed(...)
```
Rapid repeated `branch=buffered` entries without any `branch=waiting` in between means the keyboard is looping on a stale buffered key.

### Confirming Bug 2 (Wrong Layer) — Stuck layer
```
[LAYER] activate layer=1
...                          <-- no "deactivate layer=1" follows
[KB-LOOP] received event=...  <-- subsequent events process on Layer 1
```

### Normal Operation (for comparison)
```
[KB-LOOP] branch=waiting
[KB-LOOP] received event=KeyboardEvent { pressed: true, pos: Key(...) }
[KB-REPORT] send START
[KB-REPORT] send DONE
[KB-LOOP] branch=waiting
```

## Possible Fixes (After Confirming Root Cause)

### Fix A: Prevent `send_report` from blocking indefinitely

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

### Fix B: Layer state safety check

Add a periodic check in the keyboard main loop that deactivates layers when no `lt!`/morse key is pending release in the held buffer.

### Fix C: Increase report_channel_size (already done)

`keyboard.toml` already changed to `report_channel_size = 32`.

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
| `rmk/src/keyboard.rs` | Main loop (line ~131), `send_report` (line ~269), `process_buffered_key` (line ~311) |
| `rmk/src/keyboard/morse.rs` | TapHold/morse timeout handling, `handle_morse_timeout` |
| `rmk/src/keymap.rs` | `activate_layer`/`deactivate_layer` (line ~294), layer cache, `get_action_with_layer_cache` |
| `rmk/src/hid.rs` | `run_writer` loop, report channel consumer, `CONNECTION_STATE` check |
| `rmk/src/channel.rs` | `KEYBOARD_REPORT_CHANNEL` definition (size from `keyboard.toml`) |
| `rmk/src/event/input/keyboard.rs` | `#[input_event(channel_size = 16)]` — PubSub channel config |
