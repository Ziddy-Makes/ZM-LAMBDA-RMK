# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ZM-LAMBDA-RMK is BLE/USB keyboard firmware for the **nRF52840** written in **embedded Rust** (`#![no_std]`). It uses the **RMK** (Rust Mechanical Keyboard) framework with **Embassy** async runtime. The hardware is a 4x4 matrix keyboard with 14 WS2812B RGB LEDs, a rotary encoder, and Li-Ion battery monitoring.

## Build & Flash Commands

```bash
# Build (release, size-optimized)
cargo build --release

# Build + flash via debug probe (overwrites bootloader)
cargo run --release

# Build + generate UF2 for Adafruit bootloader (no probe needed)
cargo build --release && cargo make uf2 --release
# Then drag the .uf2 file to the USB drive that appears in bootloader mode

# Attach to running device for logs (after UF2 flash)
probe-rs attach --chip nRF52840_xxAA target/thumbv7em-none-eabihf/release/ZM-LAMBDA-RMK

# Recovery when locked out
nrfjprog --recover
```

**Target triple:** `thumbv7em-none-eabihf` (Cortex-M4F, bare metal)
**Logging:** `defmt` over RTT. Set level via `DEFMT_LOG` env var (default: `info`).

## Architecture

### Module Structure

- `src/main.rs` — Entry point: peripheral init, BLE/SDC stack setup, spawns all concurrent tasks
- `src/keymap.rs` — Key layout (4x4, 8 layers, 1 encoder), tapdance/macro config. Constants: `ROW`, `COL`, `NUM_LAYER`, `NUM_ENCODER`
- `src/macros.rs` — `config_matrix_pins_nrf!` macro for GPIO matrix pin initialization
- `src/led/status_controller.rs` — LED status indicator using RMK controller pattern
- `src/led/startup_animation.rs` — Boot animation sequence
- `src/vial.rs` — Auto-generated at build time from `vial.json` by `build.rs`

### RMK Controller Pattern

Controllers subscribe to events using the `#[controller(subscribe = [...])]` macro. This generates event routing boilerplate. Handler methods must follow the naming convention:

```
EventTypeName → on_{snake_case}_event()
```

Example: `BleStateChangeEvent` → `on_ble_state_change_event()`. See `docs/RMK-Controller-Changes-7Feb2026.md` for full API reference.

The `PollingController` trait provides a periodic `update()` callback (e.g., 700ms for LED blinking).

### Concurrency Model

All tasks run concurrently via Embassy async on a single-threaded executor:
```rust
join(
    run_all!(matrix, encoder, adc_device, batt_proc, keyboard, status_led),
    run_rmk(&keymap, driver, &stack, &mut storage, rmk_config),
)
```

### BLE Stack

Uses Nordic's Softdevice Controller (`nrf-sdc`) + Multiprotocol Service Layer (`nrf-mpsl`) — not the legacy SoftDevice. The BLE stack is built via `rmk::ble::build_ble_stack()`. Supports 3 BLE profiles with bond storage.

### Memory Layout (`memory.x`)

With Adafruit bootloader (current): Flash starts at `0x1000`, RAM at `0x20000008`.
Without bootloader: Flash at `0x0`, RAM at `0x20000000`.
Storage region: `0xA0000`, 12 sectors × 4KB = 48KB for keymap/bonds/macros.

## Key Dependencies (pinned to git revisions)

- **RMK** — `rev ca38784`, features: `async_matrix`, `nrf52840_ble`, `adafruit_bl`, `controller`
- **nrf-sdc / nrf-mpsl** — `rev 11d5c3c` from `alexmoon/nrf-sdc`
- **ws2812-spi** — Custom fork `aziddy/ws2812-spi-rs`, branch `nrf52840_at4Mhz_support` (4MHz SPI bit patterns)

## Hardware Reference

Pin mappings and schematic details are in `docs/ABOUT-ZM-LAMBDA.md`. Key pins:
- **Matrix:** Columns (output) P0_15, P0_11, P0_12, P1_09 / Rows (input) P0_17, P0_20, P0_22, P0_24
- **LEDs:** SPI3 data P0_26, power MOSFET P0_29
- **Encoder:** P0_08 (A), P0_06 (B)
- **Battery ADC:** P0_04 (external voltage divider)
- **DC/DC regulators must remain disabled** — no LC filter on board

## Configuration Files

- `keyboard.toml` — RMK config (macro space, vial channel size)
- `vial.json` — Vial/VIA keyboard definition (layout, custom keycodes). Compiled into firmware by `build.rs`
- `Makefile.toml` — cargo-make tasks for UF2 generation
- `.cargo/config.toml` — target, runner (`probe-rs`), linker (`flip-link`)
