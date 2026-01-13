# BLE LED Advertising Fix

## Problem

When switching to BLE mode (pressing User6/USB_BLE_SW), the LED profile indicators were not blinking to show advertising status, even though BLE was functioning correctly.

### Symptoms
- Startup animation worked ✅
- Battery indicator worked ✅
- BLE profile LEDs didn't blink when advertising ❌
- Logs showed `"BLE priority mode, running USB keyboard while advertising"` repeatedly
- No `"Advertising - Custom Controller"` logs appeared

## Root Cause

When USB is connected and the keyboard switches to BLE mode, the `StatusLedController` was waiting for a `ControllerEvent::BleState(profile, Advertising)` event that never arrived.

### Why the Event Wasn't Sent

Looking at RMK's BLE implementation (`~/.cargo/git/.../rmk/src/ble/mod.rs`):

1. The `advertise()` function creates an advertising future
2. The `BleState::Advertising` event is sent **inside** the advertise function at line 765
3. In USB+BLE dual mode, the advertising future is created but doesn't progress far enough to send the event before the event loop restarts
4. The `ControllerEvent::ConnectionType(1)` event **IS** sent immediately when switching to BLE mode

### Evidence from Logs

```
[INFO ] Switching connection type to: 1
[INFO ] Sending ControllerEvent: ConnectionType(1)  ← This WAS sent
[INFO ] BLE priority mode, running USB keyboard while advertising
[INFO ] BLE priority mode, running USB keyboard while advertising
... (repeats)
```

But we never saw:
```
[INFO ] Sending ControllerEvent: BleState(0, Advertising)  ← This was NEVER sent
```

## Solution

Modified `StatusLedController` in `src/led/status_controller.rs` to handle the `ConnectionType` event in addition to `BleState` events.

### Changes Made

#### 1. Added ConnectionType Handler (Lines 143-158)

```rust
ControllerEvent::ConnectionType(conn_type) => {
    info!("ConnectionType changed: {}", conn_type);
    // 0 = USB, 1 = BLE
    if conn_type == 1 {
        // BLE mode - start advertising indicator
        info!("BLE mode activated - starting advertising indicator");
        self.should_blink = true;
    } else {
        // USB mode - turn off BLE indicators
        info!("USB mode - stopping BLE indicators");
        self.should_blink = false;
        if !self.is_showing_battery {
            self.clear_all_leds();
        }
    }
}
```

**Why this works:**
- `ConnectionType` events are sent immediately when switching modes
- This event is reliable in USB+BLE dual mode
- The controller starts blinking right away when BLE mode is activated

#### 2. Added BleProfile Handler (Lines 180-183)

```rust
ControllerEvent::BleProfile(profile) => {
    info!("BLE Profile changed to: {}", profile);
    self.current_ble_profile = profile;
}
```

**Purpose:**
- Tracks which BLE profile (0-7) is currently active
- Ensures the correct LED blinks when switching between profiles

#### 3. Improved LED Functions (Lines 41-85)

Added bounds checking and error handling:

```rust
fn blink_ble_profile_led_blue(&mut self) {
    // ...
    // Bounds check to prevent panic
    let profile_index = (self.current_ble_profile as usize).min(N - 1);
    data[profile_index] = RGB8 { r: 0, g: 0, b: 70 };

    match self.ws2812.write(data.iter().cloned()) {
        Ok(_) => {
            info!("Successfully wrote LED data");
            self.leds_on = true;
        }
        Err(_) => {
            info!("Failed to write LED data");
        }
    }
}
```

**Improvements:**
- Prevents array index out of bounds if profile number >= NUM_LEDS (14)
- Logs success/failure of LED write operations
- Better debugging capability

#### 4. Enhanced Logging (Lines 210-219)

```rust
async fn update(&mut self) {
    if self.should_blink && !self.is_showing_battery {
        info!(
            "Update: should_blink={}, leds_on={}, profile={}",
            self.should_blink, self.leds_on, self.current_ble_profile
        );
        // ... blinking logic
    }
}
```

## Event Flow

### Before Fix
1. User presses User6 (USB_BLE_SW)
2. `ControllerEvent::ConnectionType(1)` sent ✅
3. `StatusLedController` ignores it ❌
4. Waits for `BleState::Advertising` that never arrives ❌
5. LEDs don't blink ❌

### After Fix
1. User presses User6 (USB_BLE_SW)
2. `ControllerEvent::ConnectionType(1)` sent ✅
3. `StatusLedController` receives it ✅
4. Sets `should_blink = true` ✅
5. LEDs start blinking immediately ✅
6. `BleProfile` events update which LED blinks ✅
7. `BleState` events still work when received ✅

## Controller Event Types Used

The `StatusLedController` now handles these `ControllerEvent` variants:

| Event | Purpose | When Sent |
|-------|---------|-----------|
| `ConnectionType(u8)` | Switch between USB (0) and BLE (1) | Immediately when mode changes |
| `BleProfile(u8)` | Track active BLE profile (0-7) | When profile switches |
| `BleState(u8, BleState)` | BLE connection state changes | Advertising, Connected, None |
| `Battery(u8)` | Battery percentage updates | Periodically from ADC |
| `Key(KeyboardEvent, KeyAction)` | Key press/release events | User7 for battery display |

## Testing

After building and flashing:

1. **Test BLE Mode Activation:**
   - Press User6 to switch to BLE mode
   - LED corresponding to active profile should blink blue
   - Log: `"ConnectionType changed: 1"` and `"BLE mode activated"`

2. **Test Profile Switching:**
   - Switch BLE profiles (User0-User7 keys)
   - Different LED should blink
   - Log: `"BLE Profile changed to: X"`

3. **Test Connection:**
   - Connect a BLE device
   - LED should blink green 4 times then stop
   - Log: `"Connected - Custom Controller - Profile: X"`

4. **Test USB Mode:**
   - Press User6 again to switch back to USB
   - LEDs should turn off
   - Log: `"USB mode - stopping BLE indicators"`

5. **Test Battery Indicator:**
   - Hold User7 while in any mode
   - Battery level should display (works independently)

## Files Modified

- `src/led/status_controller.rs` - Main fix implementation

## Compatibility

- Works in both USB-only and USB+BLE dual mode
- Preserves all existing functionality (battery indicator, connection status)
- Backward compatible with BleState events when they arrive

## Future Improvements

Potential enhancements:

1. **Different blink patterns** for advertising vs searching for known device
2. **Color coding** by profile (instead of just position)
3. **Fade effect** instead of hard on/off blinking
4. **Indicate advertising timeout** (different pattern after 5 minutes)

## References

- RMK ControllerEvent enum: `~/.cargo/git/.../rmk/src/event.rs:149-180`
- RMK BLE advertising: `~/.cargo/git/.../rmk/src/ble/mod.rs:718-781`
- Controller trait: `~/.cargo/git/.../rmk/src/controller/mod.rs`
