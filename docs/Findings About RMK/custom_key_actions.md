# Custom Key Actions in RMK

## Overview
RMK provides multiple ways to detect and handle custom key presses. The most straightforward approach is using **USER keys** (User0-User31), which are special keycodes designed for custom functionality.

## USER Key Codes

### Available USER Keys
RMK defines 32 USER keycodes (`User0` through `User31`) located at addresses `0x840` through `0x85F`:

```rust
// From rmk-types/src/keycode.rs:830-861
User0 = 0x840,
User1 = 0x841,
User2 = 0x842,
// ... up to ...
User31 = 0x85F,
```

### Using USER Keys in Your Keymap

To add a USER key to your keymap, use the `k!()` macro with the keycode:

```rust
// In your keymap.rs
use rmk::types::action::KeyAction;
use rmk::{k, a, layer};

pub const fn get_default_keymap() -> [[[KeyAction; COL]; ROW]; NUM_LAYER] {
    [
        layer!([
            [k!(User0), k!(User1), k!(A), k!(B), /* ... */],
            // ... more rows
        ]),
        // ... more layers
    ]
}
```

## Handling USER Key Presses

### Method 1: Modify RMK Source (Not Recommended)

The default `process_user()` function in RMK is located at [keyboard.rs:1715-1769](~/.cargo/git/checkouts/rmk-cd9707f7f2031ce2/f0872fa/rmk/src/keyboard.rs#L1715).

By default, USER keys are reserved for BLE profile switching (when the `_ble` feature is enabled):
- `User0-User7`: Switch to specific BLE profiles
- `User8`: Next profile
- `User9`: Previous profile
- `User10`: Clear profile
- `User11`: Toggle connection
- `User12+4` (User16): Clear peer (with 5s hold)

### Method 2: Using the Controller Pattern (Recommended)

RMK uses a **Controller pattern** where keyboard events are broadcast to controllers that can react to them. This is the cleanest way to add custom functionality without modifying RMK source.

#### Step 1: Subscribe to Controller Events

The keyboard broadcasts `ControllerEvent` messages through `CONTROLLER_CHANNEL`. Key events are sent as:

```rust
ControllerEvent::Key(KeyboardEvent, KeyAction)
```

Reference: [keyboard.rs:722](~/.cargo/git/checkouts/rmk-cd9707f7f2031ce2/f0872fa/rmk/src/keyboard.rs#L722)

#### Step 2: Create a Custom Controller

Create a controller struct that implements the `Controller` trait:

```rust
use rmk::channel::{CONTROLLER_CHANNEL, ControllerSub};
use rmk::controller::Controller;
use rmk::event::{ControllerEvent, KeyboardEvent};
use rmk::types::action::KeyAction;
use rmk::types::keycode::KeyCode;

pub struct CustomKeyController {
    sub: ControllerSub,
}

impl CustomKeyController {
    pub fn new() -> Self {
        Self {
            sub: CONTROLLER_CHANNEL.subscriber().unwrap(),
        }
    }
}

impl Controller for CustomKeyController {
    type Event = ControllerEvent;

    async fn process_event(&mut self, event: Self::Event) {
        if let ControllerEvent::Key(keyboard_event, key_action) = event {
            // Check if it's a USER key
            if let KeyAction::Single(rmk::types::action::Action::Key(keycode)) = key_action {
                match keycode {
                    KeyCode::User0 => {
                        if keyboard_event.pressed {
                            // User0 was pressed - run your custom code here
                            defmt::info!("User0 pressed!");
                            // Your custom logic here
                        } else {
                            // User0 was released
                            defmt::info!("User0 released!");
                        }
                    }
                    KeyCode::User1 => {
                        if keyboard_event.pressed {
                            defmt::info!("User1 pressed!");
                            // Your custom logic for User1
                        }
                    }
                    // Handle more USER keys as needed
                    _ => {}
                }
            }
        }
    }
}
```

#### Step 3: Spawn the Controller Task

In your `main.rs`, spawn your custom controller as an async task:

```rust
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // ... your initialization code ...

    // Create and spawn your custom key controller
    let custom_controller = CustomKeyController::new();
    spawner.spawn(run_custom_controller(custom_controller)).unwrap();

    // ... rest of your code ...
}

#[embassy_executor::task]
async fn run_custom_controller(mut controller: CustomKeyController) -> ! {
    use rmk::controller::Controller;
    controller.run().await
}
```

## Detecting Any Existing KeyCode

You can also detect any standard KeyCode (not just USER keys) using the same controller pattern:

```rust
impl Controller for CustomKeyController {
    type Event = ControllerEvent;

    async fn process_event(&mut self, event: Self::Event) {
        if let ControllerEvent::Key(keyboard_event, key_action) = event {
            if let KeyAction::Single(rmk::types::action::Action::Key(keycode)) = key_action {
                match keycode {
                    KeyCode::A => {
                        if keyboard_event.pressed {
                            defmt::info!("A key pressed!");
                            // Your custom logic
                        }
                    }
                    KeyCode::Escape => {
                        if keyboard_event.pressed {
                            defmt::info!("Escape pressed!");
                            // Your custom logic
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
```

## Event Flow

Here's how key events flow through RMK:

1. **Key Press Detected** ’ Matrix scan detects key state change
2. **KeyboardEvent Created** ’ Event with position (row, col) and pressed state
3. **KeyAction Retrieved** ’ Keymap is consulted to get the KeyAction for this key
4. **Controller Notification** ’ `ControllerEvent::Key(event, action)` is broadcast via `CONTROLLER_CHANNEL`
5. **Action Processing** ’ The keyboard processes the action (send HID report, layer switch, etc.)

Your custom controller receives the event at step 4, **before** the action is fully processed, allowing you to react or even modify behavior.

## Key References

- **KeyCode enum**: [rmk-types/src/keycode.rs:14-862](~/.cargo/git/checkouts/rmk-cd9707f7f2031ce2/f0872fa/rmk-types/src/keycode.rs#L14)
- **KeyAction enum**: [rmk-types/src/action.rs:232-246](~/.cargo/git/checkouts/rmk-cd9707f7f2031ce2/f0872fa/rmk-types/src/action.rs#L232)
- **Action enum**: [rmk-types/src/action.rs:288-324](~/.cargo/git/checkouts/rmk-cd9707f7f2031ce2/f0872fa/rmk-types/src/action.rs#L288)
- **process_user() function**: [rmk/src/keyboard.rs:1715-1769](~/.cargo/git/checkouts/rmk-cd9707f7f2031ce2/f0872fa/rmk/src/keyboard.rs#L1715)
- **ControllerEvent enum**: [rmk/src/event.rs:149-159](~/.cargo/git/checkouts/rmk-cd9707f7f2031ce2/f0872fa/rmk/src/event.rs#L149)
- **Controller trait**: [rmk/src/controller/mod.rs](~/.cargo/git/checkouts/rmk-cd9707f7f2031ce2/f0872fa/rmk/src/controller/mod.rs)

## Example: Toggle LED on User0 Press

```rust
use embassy_nrf::gpio::{Output, Level, OutputDrive};

pub struct LedToggleController {
    sub: ControllerSub,
    led: Output<'static>,
    led_state: bool,
}

impl LedToggleController {
    pub fn new(led_pin: impl embassy_nrf::gpio::Pin) -> Self {
        Self {
            sub: CONTROLLER_CHANNEL.subscriber().unwrap(),
            led: Output::new(led_pin, Level::Low, OutputDrive::Standard),
            led_state: false,
        }
    }
}

impl Controller for LedToggleController {
    type Event = ControllerEvent;

    async fn process_event(&mut self, event: Self::Event) {
        if let ControllerEvent::Key(keyboard_event, key_action) = event {
            if let KeyAction::Single(rmk::types::action::Action::Key(KeyCode::User0)) = key_action {
                if keyboard_event.pressed {
                    // Toggle LED on User0 press
                    self.led_state = !self.led_state;
                    if self.led_state {
                        self.led.set_high();
                    } else {
                        self.led.set_low();
                    }
                }
            }
        }
    }
}
```

## Summary

- **Use USER keys (User0-User31)** for custom functionality
- **Implement a Controller** that subscribes to `CONTROLLER_CHANNEL`
- **Match on KeyCode** in your controller's `process_event()` method
- **Check `keyboard_event.pressed`** to detect press vs. release
- **Spawn your controller** as an async task using Embassy

This approach keeps your custom logic separate from RMK internals and makes it easy to maintain when updating RMK versions.
