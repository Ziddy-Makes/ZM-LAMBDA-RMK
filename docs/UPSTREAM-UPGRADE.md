# RMK upstream upgrade — 25 October 2026

This firmware consumes RMK as a dependency. Branch `upgrade/rmk-80c0bda7` ports
the existing application from upstream `bf9922b6287ad14e4ce661e3f0794fc1c2c1ffb1`
to `80c0bda72fee2fc75e1d27d08012435e71983247` (22 October 2026), including the
changes after the v0.9.0 release.

## Integration changes

- Pin RMK directly to `rmk-rs/rmk` and remove the sibling `rmk-zm-patch` dependency.
- Update Embassy nRF to 0.11, bt-hci to 0.10, and nrf-sdc/nrf-mpsl to 0.4.
- Let `BleTransport` own its BLE stack and attach both transports to the shared
  `HostService`.
- Use `UsbTransport::builder` for additional USB descriptor space. This replaces
  the local 512-byte buffer patch while retaining the Vial interface.
- Adapt battery ADC construction to device IDs and millivolt readings. Keep the
  P0_02 input, default SAADC settings, and 1000:1400 voltage-divider ratio.
- Use the nRF hardware-derived default serial number, matching the serial that
  the previous RMK version actually reported over USB and BLE.
- Represent the existing text macro as a static `MacroOp` table.
- Run the upstream nRF watchdog alongside the other components, and handle the
  battery-display key using explicit press/release events.

The board retains its 4×4 matrix, eight layers, encoder, 14 LEDs and custom 4MHz
SPI driver, Adafruit UF2 bootloader layout, and 48KiB storage at
`0xA0000..0xAC000`. Both DC/DC regulators must stay disabled because this board
has no regulator LC filters.

## First flash

Back up any Vial keymap, macro, and settings changes before flashing. Upstream's
new storage schema check erases the full configured storage region when it
finds the old format, including the BLE bonds. Compiled-in defaults are then
loaded. `clear_storage = false` does not bypass a schema mismatch.

After flashing, forget the keyboard on each Bluetooth host and pair again.
The HID report IDs have also changed since the previous pinned revision.

The hardware has no configured charging-state GPIO. Battery levels remain
available; a charging indicator requires a real charger-state input.

## Build

The migration builds using Rust 1.91.0 for `thumbv7em-none-eabihf`:

```sh
cargo build --release --locked
cargo make uf2 --release
```

Both commands completed successfully. The linked flash image ends at `0x606AC`,
below the storage start at `0xA0000`. The generated UF2 also fits below storage.

Use the existing `cargo make uf2 --release` workflow to generate an Adafruit
UF2 image. Flashing and USB/BLE/LED behavior require validation on the board.
