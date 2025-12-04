## Building & Flashing
### A) Build & Generate UF2 File (w/ Adafruit Bootloader on nRF):

**Prerequisites**
1) Adafruit UF2 bootloader installed and working properly on nRF52840
	- Can check its working correctly by seeing if UF2 storage devices shows on computer
2) The right memory locations are defined in `memory.x` - [Link to RMK Guide Here](https://rmk.rs/guide/user_guide/3_flash_firmware.html)
	- `FLASH : ORIGIN = 0x00001000, LENGTH = 1020K`
	- `RAM : ORIGIN = 0x20000008, LENGTH = 255K`

> [!INFO] We offset our ***RMK*** program to ***not overwrite the start*** of Flash & RAM address space to ***prevent overwriting bootloader***

```bash
# Clean Build Artifacts
cargo clean

# Build
cargo build --release

# Convert to UF2
cargo make uf2 --release

# COMBINED: Build & UF2 Generate
cargo build --release && cargo make uf2 --release
```

> [!question] **TIP:** Assign `bootloader` keycode to a key, so you don't have keep double tapping reset like a madman
> 

#### Debug / Logs with Option A)
You can
1) Hookup your board to the **nRF52-DK / SWD** after UF2 uploading
2) Run the following command below
```bash
probe-rs attach --chip nRF52840_xxAA target/thumbv7em-none-eabihf/release/ZM9K-BLE-RMK
```
To still view **RMK** logs even with UF2 uploaded firmware ✅


### B) Directly Flash & Log Attach w/ SWD
1) **nRF52-DK / SWD** connected to your board
2) The right memory locations are defined in `memory.x` - [Link to RMK Guide Here](https://rmk.rs/guide/user_guide/3_flash_firmware.html)
	- `FLASH : ORIGIN = 0x00000000, LENGTH = 1024K`
	- `RAM : ORIGIN = 0x20000000, LENGTH = 256K`

> [!FAILURE] This will **Overwrite** your Adafruit bootloader *(preventing UF2 upload)*
> - ✅ But will make development testing faster
> - ❌ Will need to Erase/Wipe & Reflash Bootloader again Later

```bash
# Builds, Erase/Flash, Attach Logs to device
cargo run --release
```
- Runs `probe-rs run --chip nRF52840_xxAA` under the hood
	- Defined in `config.toml`
  


## Other Info

### What does `cargo run` /  `probe-rs run` do?
Both
- The part of `cargo run --release` that runs `probe-rs run ..` 
- `probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabihf/release/...`

> [!WARNING] Don't need command argument `--base-address 0x0...` with `probe-rs` `run` or `download`
> - Default `probe-rs` `run/download` uses the ELF (thats generated on `cargo build ..`) that contains the addresses from your `memory.x`
> - You need to specifically use a `.hex` output to require using `--base-address`

### Erase, Flash, but don't Attach to Log Out

```bash
probe-rs download --chip nRF52840_xxAA target/thumbv7em-none-eabihf/release/ZM9K-BLE-RMK
```

### When your completely locked out of the nRF52840
Run this bad boy
```bash
nrfjprog --recover
```

  
### 🚧 Concerns

#### 🤔🧠 Would be nice to upload with SWD, but still have bootloader intact

> *Does `cargo run --release` or `probe-rs run ...` with the `memory.x` config set to accommodate the Adafruit bootloader. Still properly work?*

> *Maybe running `probe-rs download ...` and then running `probe-rs attach` can work with the Adafruit Bootloader `memory.x` config?*