# RMK features and improvements since bf9922b

This summary covers the upstream changes included by the October 2026 upgrade
of ZM-LAMBDA-RMK:

| Baseline | Upstream revision |
| --- | --- |
| Previous RMK dependency, May 2026 | `bf9922b6287ad14e4ce661e3f0794fc1c2c1ffb1` |
| Upgraded RMK dependency, 22 October 2026 | `80c0bda72fee2fc75e1d27d08012435e71983247` |

There are 659 upstream commits between these revisions, including merges.
The range includes v0.9.0, published on 29 August 2026, and subsequent
unreleased changes on `main`. It describes the pinned upgrade rather than
tracking future upstream changes. [Compare the revisions](https://github.com/rmk-rs/rmk/compare/bf9922b6287ad14e4ce661e3f0794fc1c2c1ffb1...80c0bda72fee2fc75e1d27d08012435e71983247).

## Improvements included in the firmware upgrade

### USB reliability

- BLE continues working when USB supplies power through a charger or charge-only
  cable, without a host configuring the USB device.
- Fixes prevent the input pipeline from freezing while a USB host is asleep.
  Remote wakeup is retried without blocking on suspended endpoints.
- Bus resets clear stale USB configuration state.
- USB boot-protocol handling is improved for BIOS/UEFI and other hosts that use
  the keyboard boot protocol.

The charging and suspend fixes shipped in v0.9.0; the bus-reset and
`SET_PROTOCOL` fixes followed on `main`.
[USB suspend fix](https://github.com/rmk-rs/rmk/pull/1035),
[current USB implementation](https://github.com/rmk-rs/rmk/blob/80c0bda7/rmk/src/usb/mod.rs).

### Bluetooth compatibility and pairing

- Android receives mouse, media, and system-control keys through a single BLE
  HID service containing the keyboard's reports.
- Holding a BLE profile key for five seconds forgets that profile's bond and
  starts pairing again.
- Connection parameters, bond validation, reconnection, and nRF clock settings
  receive fixes. The nRF52 RC clock is declared at its specified 500 ppm.

These improvements are part of the
[v0.9.0 release](https://github.com/rmk-rs/rmk/releases/tag/rmk-v0.9.0).

### Key behavior and input handling

- Quick-tap repeat lets a recently tapped tap-hold key immediately repeat its
  tap action when pressed again within the configured window.
- Flow-tap can be overridden per timing profile, and tap-hold/morse
  configuration supports nested actions.
- Fixes address stuck or overlapping combos, dropped key presses, one-shot
  modifiers, tap-hold release handling, and encoder sampling.
- Tap-hold actions reference shared timing profiles, reducing keymap RAM use.

The behavior options and profile sharing shipped in v0.9.0; additional input
fixes followed on `main`. See the
[changelog at the upgraded revision](https://github.com/rmk-rs/rmk/blob/80c0bda7/rmk/CHANGELOG.md).

### Macros and Vial

- Full extended Vial macro formats are supported.
- Vial user keycodes `User16` through `User31` retain their identities instead
  of aliasing to `User0` through `User15`.
- The subsequent macro API uses `MacroOp` lists. Macro operations can execute
  single actions such as modifiers and layer changes.
- `MacroOp::PauseForRelease` pauses a macro until its trigger key is released,
  then runs the remaining operations.

This project now represents its existing text macro using the new API. See the
[macro changes](https://github.com/rmk-rs/rmk/blob/80c0bda7/rmk/CHANGELOG.md)
and the local [upgrade notes](UPSTREAM-UPGRADE.md).

### Battery reporting and status access

- Battery ADC events use millivolts; the battery processor applies the voltage
  divider ratio. Charger-state handling is assembled separately from the BLE
  transport.
- Current BLE and connection status can be read directly through public status
  functions.
- `BleStatus::bonded` distinguishes a paired but disconnected profile from an
  empty profile. Custom LEDs and displays can use this information.

The ADC migration is integrated into this firmware. The new status functions
and bond flag are available for future LED behavior changes. This board has
no configured charger-state GPIO, so charging indications require a real
charger-state input. See the
[battery implementation](https://github.com/rmk-rs/rmk/blob/80c0bda7/rmk/src/input_device/battery.rs)
and [status additions](https://github.com/rmk-rs/rmk/blob/80c0bda7/rmk/CHANGELOG.md).

## Optional new features

These capabilities require additional configuration or hardware; they are not
enabled by this project's current upgrade.

| Feature | What it adds | Requirements |
| --- | --- | --- |
| BLE USB dongle | A USB receiver relays wireless keyboard input and Vial configuration to a computer. | A compatible nRF receiver and dongle firmware/configuration. |
| Rynk, experimental | Native configuration over USB/BLE, with access to keymaps, behaviors, macros, live status, and device management. | Enable Rynk in place of Vial and use compatible host software. |
| USB firmware updates, experimental | DFU firmware updates with a compatible bootloader; later work adds an external SPI flash option for the update image. | Appropriate DFU features and bootloader/flash layout. The existing Adafruit UF2 workflow remains this project's flashing method. |
| Pointing-device modes | Per-layer cursor, scroll, precision, and caret modes, plus automatic mouse-layer activation. | A supported pointing device and configuration. |
| Pointer acceleration and touchpad gestures | Acceleration, fractional cursor speed, and touchpad gestures mapped to per-layer actions. | Supported hardware and configuration; these additions are from the work after v0.9.0. |

See the [v0.9.0 feature list](https://github.com/rmk-rs/rmk/releases/tag/rmk-v0.9.0)
and the [subsequent changelog](https://github.com/rmk-rs/rmk/blob/80c0bda7/rmk/CHANGELOG.md)
for the upstream details.

## Before flashing

The upstream storage schema change resets saved Vial settings and BLE bonds on
first boot. Back up settings and plan to pair Bluetooth hosts again. The build
and UF2 generation pass; behavior still requires validation on the board.
See [upgrade and first-flash notes](UPSTREAM-UPGRADE.md) for the migration details.
