# RMK Flash Storage: How It Works

## Overview

RMK uses the `sequential-storage` crate (v5.0.1) to persist keyboard configuration, keymaps, macros, and BLE bonding data to flash memory. This document explains how the storage system works and how to add custom data.

## Storage Architecture

### Three-Layer System

1. **Flash Hardware Layer**: Uses `embedded_storage_async::nor_flash::NorFlash` trait
   - On nRF52: `Flash::take(mpsl, p.NVMC)` from `nrf_mpsl`
   - Wrapped with `BlockingAsync` adapter for async operations

2. **Sequential-Storage Layer**: Key-value storage using `sequential-storage::map`
   - `fetch_item<K, V, S>()` - Read data
   - `store_item<K, V, S>()` - Write data
   - `fetch_all_items()` - Iterate all items
   - `erase_all()` - Clear storage

3. **RMK Storage Manager**: `Storage<F, ROW, COL, NUM_LAYER, NUM_ENCODER>`
   - Runs as async task processing operations via channels
   - Handles serialization/deserialization
   - Manages storage lifecycle

## Storage Configuration

Your [main.rs:185-190](../../../src/main.rs#L185-L190) shows:

```rust
let storage_config = StorageConfig {
    start_addr: 0xA0000,     // Flash start address (must be sector-aligned)
    num_sectors: 12,         // Number of 4KB sectors (48KB total)
    clear_storage: false,    // Erase all on boot?
    clear_layout: false,     // Reset keymap only?
};
```

**Important**:
- `num_sectors` must be e 2 (sequential-storage needs 2 sectors minimum)
- Sectors are 4KB on nRF52840
- Your config: 12 sectors = 48KB storage from 0xA0000 to 0xAC000

**Location**: RMK source: `~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/config/mod.rs:165-186`

## Storage Keys System

RMK uses `u32` keys to identify different data types:

### System Keys (0x00-0x0F)
- `0x00` - StorageConfig (metadata)
- `0x01` - KeymapConfig
- `0x02` - LayoutConfig (default layer, layout options)
- `0x03` - BehaviorConfig (timeouts, tap-dance, etc.)
- `0x04` - MacroData
- `0x05` - ComboData
- `0x06` - ConnectionType
- `0x07` - EncoderKeys
- `0x08` - ForkData
- `0x09` - MorseData

### Dynamic Key Ranges
- `0x1000-0x1FFF` - Keymap keys (per layer/row/col)
- `0x2000-0x2FFF` - BLE bond info
- `0x3000-0x3FFF` - Combo definitions
- `0x4000-0x4FFF` - Encoder configs
- `0x5000-0x5FFF` - Fork definitions
- `0x6000-0x6FFF` - Peer addresses (split keyboard)
- `0x7000-0x7FFF` - Morse/tap-dance configs

**Special Keys**:
- `0xED` - PeerAddress
- `0xEE` - ActiveBleProfile
- `0xEF` - BleBondInfo

**Location**: RMK source: `~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/storage/mod.rs:82-140`

## How Storage Operations Work

### Initialization

When you call `initialize_encoder_keymap_and_storage()` in [main.rs:212](../../../src/main.rs#L212):

```rust
let (keymap, mut storage) = initialize_encoder_keymap_and_storage(
    &mut default_keymap,
    &mut encoder_map,
    flash,              // Flash peripheral
    &storage_config,    // Your config
    &mut behavior_config,
    &mut key_config,
).await;
```

**What happens**:
1. Validates `num_sectors >= 2`
2. Calculates storage range: `0xA0000..(0xA0000 + 12*4096)` = `0xA0000..0xAC000`
3. Checks if storage is initialized (reads StorageConfig key)
4. If not initialized OR `clear_storage=true`:
   - Erases entire range
   - Writes StorageConfig with `enable=true` and build hash
   - Stores default LayoutConfig
   - Stores BehaviorConfig
   - Stores entire keymap (all layers)
   - Stores encoder configs
5. If already initialized:
   - Loads BehaviorConfig from flash
   - Loads keymap using `fetch_all_items()` iterator
   - Loads macros, combos, forks, morse configs

**Location**: RMK source: `~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/lib.rs:133-190`

### Runtime Storage Task

The `storage` object runs as a background async task passed to `run_rmk()` in [main.rs:282](../../../src/main.rs#L282):

```rust
run_rmk(&keymap, driver, &stack, &mut storage, rmk_config)
```

The storage task:
1. Waits for messages on `FLASH_CHANNEL`
2. Processes `FlashOperationMessage` variants
3. Signals completion via `FLASH_OPERATION_FINISHED`

**Location**: RMK source: `~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/storage/mod.rs:635-884`

### Channel-Based Communication

Storage operations use Embassy channels:

```rust
// Send operation (from anywhere in your code)
FLASH_CHANNEL.send(FlashOperationMessage::LayoutOptions(value)).await;

// Wait for completion (optional)
FLASH_OPERATION_FINISHED.wait().await;
```

**Available Message Types**:
- `ProfileInfo(ProfileInfo)` - Save BLE bond
- `ActiveBleProfile(u8)` - Save active profile
- `Reset` - Erase all storage
- `ResetLayout` - Reset keymap only
- `ClearSlot(u8)` - Clear BLE bond slot
- `LayoutOptions(u32)` - Save layout options
- `DefaultLayer(u8)` - Save default layer
- `VialMessage(KeymapData)` - Save keymap/macro changes
- `ConnectionType(u8)` - Save connection type
- `ComboTimeout(u16)`, `OneShotTimeout(u16)`, etc. - Behavior settings

**Location**: RMK source: `~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/storage/mod.rs:38-80`

## Data Structures

### StorageData Enum

The main type stored to flash:

```rust
pub(crate) enum StorageData {
    StorageConfig(LocalStorageConfig),
    LayoutConfig(LayoutConfig),
    BehaviorConfig(BehaviorConfig),
    ConnectionType(u8),
    VialData(KeymapData),
    PeerAddress(PeerAddress),
    BondInfo(ProfileInfo),
    ActiveBleProfile(u8),
}
```

### KeymapData Enum (Via/Vial)

```rust
pub(crate) enum KeymapData {
    Macro([u8; MACRO_SPACE_SIZE]),
    KeymapKey(KeymapKey),
    Encoder(EncoderConfig),
    Combo(ComboData),
    Fork(ForkData),
    Morse(u8, Morse),
}
```

## Serialization Format

All stored data implements `sequential_storage::map::Value` trait:

```rust
impl Value<'_> for StorageData {
    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, SerializationError> {
        // First byte is ALWAYS the StorageKeys enum value (type identifier)
        buffer[0] = StorageKeys::LayoutConfig as u8;

        // Then serialize data using BigEndian byte order
        buffer[1] = data.default_layer;
        BigEndian::write_u32(&mut buffer[2..6], data.layout_options);

        // Return total bytes written
        Ok(6)
    }

    fn deserialize_from(buffer: &[u8]) -> Result<Self, SerializationError> {
        // Read first byte to determine type
        if let Some(key_type) = StorageKeys::from_u8(buffer[0]) {
            match key_type {
                StorageKeys::LayoutConfig => {
                    let default_layer = buffer[1];
                    let layout_options = BigEndian::read_u32(&buffer[2..6]);
                    Ok(StorageData::LayoutConfig(LayoutConfig {
                        default_layer,
                        layout_options
                    }))
                }
                // ... other cases
            }
        }
    }
}
```

**Key Points**:
- First byte = type identifier (`StorageKeys` enum)
- Data uses BigEndian encoding via `byteorder` crate
- Buffer must be large enough for largest value
- Default buffer: 256 bytes (aligned to 32 bytes), or larger for macros

**Location**: RMK source: `~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/storage/mod.rs:202-424`

## Sequential-Storage Characteristics

### How It Works

- **Append-Only**: New writes append data; old values marked as invalid
- **Automatic Compaction**: When sector fills, data migrated to next sector
- **Power-Loss Safe**: Atomic sector switching with checksums
- **Minimal Erases**: Only erases when absolutely necessary (wear leveling)
- **No Cache Required**: `NoCache::new()` sufficient for most uses

### Core API

```rust
// Read
let data = fetch_item::<u32, StorageData, _>(
    &mut flash,
    storage_range.clone(),
    &mut NoCache::new(),
    &mut buffer,
    &key,
).await?;

// Write
store_item(
    &mut flash,
    storage_range.clone(),
    &mut NoCache::new(),
    &mut buffer,
    &key,
    &value,
).await?;

// Erase all
erase_all(&mut flash, storage_range).await?;
```

## Reading Storage Examples

### Example 1: Read BLE Bond Info

From RMK source (`~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/storage/mod.rs:1142-1157`):

```rust
pub(crate) async fn read_trouble_bond_info(
    &mut self,
    slot_num: u8
) -> Result<Option<ProfileInfo>, ()> {
    let read_data = fetch_item::<u32, StorageData, _>(
        &mut self.flash,
        self.storage_range.clone(),
        &mut NoCache::new(),
        &mut self.buffer,
        &get_bond_info_key(slot_num),  // Key = 0x2000 + slot_num
    )
    .await
    .map_err(|e| print_storage_error::<F>(e))?;

    if let Some(StorageData::BondInfo(info)) = read_data {
        Ok(Some(info))
    } else {
        Ok(None)
    }
}
```

### Example 2: Read All Keymap Keys

From RMK source (`~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/lib.rs:157-177`):

```rust
// Iterate all stored items
let all_items = fetch_all_items::<u32, StorageData, _>(
    &mut flash,
    storage_range.clone(),
    &mut NoCache::new(),
    &mut buffer,
)
.await?;

// Process each item
while let Some((key, item)) = all_items.next(&mut flash, &mut buffer).await? {
    match item {
        StorageData::VialData(KeymapData::KeymapKey(k)) => {
            // Extract layer, row, col from key
            let (layer, row, col) = parse_keymap_key(key);
            keymap[layer][row][col] = k.action;
        }
        // ... handle other types
        _ => {}
    }
}
```

## Writing Storage Examples

### Example 1: Via Channel (High-Level, Recommended)

From RMK source (`~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/ble/profile.rs:229-232`):

```rust
use rmk::channel::FLASH_CHANNEL;
use rmk::storage::FlashOperationMessage;

// Save BLE profile
FLASH_CHANNEL
    .send(FlashOperationMessage::ProfileInfo(profile_info))
    .await;

// Wait for completion (optional)
FLASH_OPERATION_FINISHED.wait().await;
```

### Example 2: Direct Storage API (Low-Level)

From RMK source (`~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/storage/mod.rs:1183-1192`):

```rust
// Write directly to storage
store_item(
    &mut self.flash,
    self.storage_range.clone(),
    &mut NoCache::new(),
    &mut self.buffer,
    &(StorageKeys::LayoutConfig as u32),     // Key
    &StorageData::LayoutConfig(layout_cfg),  // Value
)
.await
.map_err(|e| print_storage_error::<F>(e))
```

### Example 3: Save Macros

From RMK source (`~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/host/via/mod.rs:275-281`):

```rust
// Get macro buffer from keymap
let buf = self.keymap.borrow_mut().behavior.keyboard_macros.macro_sequences;

// Save to flash via channel
FLASH_CHANNEL
    .send(FlashOperationMessage::VialMessage(KeymapData::Macro(buf)))
    .await;
```

## Adding Your Own Custom Data

### Step-by-Step Guide

#### 1. Choose an Unused Key Value

Pick a value not used by existing `StorageKeys`. Good range: `0x10-0xFF` or custom ranges like `0x8000+`.

#### 2. Define Your Data Structure

```rust
// In your code (e.g., src/my_custom_storage.rs)
#[derive(Debug, Clone, Copy)]
pub struct MyCustomData {
    pub field1: u8,
    pub field2: u16,
    pub field3: u32,
}
```

#### 3. Add to StorageKeys Enum (Fork RMK)

**If modifying RMK source** (`~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/storage/mod.rs:86-110`):

```rust
pub(crate) enum StorageKeys {
    // ... existing keys
    MyCustomData = 0x10,  // Pick unused value
}

impl StorageKeys {
    pub(crate) fn from_u8(value: u8) -> Option<Self> {
        match value {
            // ... existing cases
            0x10 => Some(StorageKeys::MyCustomData),
            _ => None,
        }
    }
}
```

#### 4. Add to StorageData Enum

```rust
pub(crate) enum StorageData {
    // ... existing variants
    MyCustomData(MyCustomData),
}
```

#### 5. Implement Serialization

Add to `Value` trait implementation:

```rust
impl Value<'_> for StorageData {
    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, SerializationError> {
        match self {
            // ... existing cases
            StorageData::MyCustomData(data) => {
                buffer[0] = StorageKeys::MyCustomData as u8;
                buffer[1] = data.field1;
                BigEndian::write_u16(&mut buffer[2..4], data.field2);
                BigEndian::write_u32(&mut buffer[4..8], data.field3);
                Ok(8)  // Total bytes written
            }
        }
    }

    fn deserialize_from(buffer: &[u8]) -> Result<Self, SerializationError> {
        if let Some(key_type) = StorageKeys::from_u8(buffer[0]) {
            match key_type {
                // ... existing cases
                StorageKeys::MyCustomData => {
                    let field1 = buffer[1];
                    let field2 = BigEndian::read_u16(&buffer[2..4]);
                    let field3 = BigEndian::read_u32(&buffer[4..8]);
                    Ok(StorageData::MyCustomData(MyCustomData {
                        field1, field2, field3
                    }))
                }
            }
        }
    }
}
```

#### 6. Add Flash Message Type (Optional, for Runtime Updates)

In `FlashOperationMessage` enum:

```rust
pub(crate) enum FlashOperationMessage {
    // ... existing variants
    MyCustomData(MyCustomData),
}
```

#### 7. Handle in Storage Task

In `Storage::run()` method:

```rust
match info {
    // ... existing cases
    FlashOperationMessage::MyCustomData(data) => {
        store_item(
            &mut self.flash,
            self.storage_range.clone(),
            &mut storage_cache,
            &mut self.buffer,
            &(StorageKeys::MyCustomData as u32),
            &StorageData::MyCustomData(data),
        )
        .await
    }
}
```

#### 8. Add Read Function

Add method to `Storage` impl:

```rust
impl<F: AsyncNorFlash, ...> Storage<F, ROW, COL, NUM_LAYER, NUM_ENCODER> {
    pub(crate) async fn read_my_custom_data(&mut self) -> Result<Option<MyCustomData>, ()> {
        let read_data = fetch_item::<u32, StorageData, _>(
            &mut self.flash,
            self.storage_range.clone(),
            &mut NoCache::new(),
            &mut self.buffer,
            &(StorageKeys::MyCustomData as u32),
        )
        .await
        .map_err(|e| print_storage_error::<F>(e))?;

        if let Some(StorageData::MyCustomData(data)) = read_data {
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
}
```

#### 9. Usage in Your Application

```rust
use rmk::channel::FLASH_CHANNEL;

// Write
let my_data = MyCustomData {
    field1: 42,
    field2: 1337,
    field3: 0xDEADBEEF,
};

FLASH_CHANNEL
    .send(FlashOperationMessage::MyCustomData(my_data))
    .await;

// Read (if you have access to Storage)
if let Ok(Some(data)) = storage.read_my_custom_data().await {
    info!("Read: {:?}", data);
}
```

### Alternative: Without Forking RMK

If you don't want to fork RMK, you can use **unused key ranges** directly with the `sequential-storage` API:

```rust
use sequential_storage::map::{fetch_item, store_item};
use sequential_storage::cache::NoCache;
use byteorder::{BigEndian, ByteOrder};

// Your custom key (pick from unused range, e.g., 0x8000+)
const MY_CUSTOM_KEY: u32 = 0x8000;

// Simple wrapper type implementing Value trait
pub struct MyStorageValue {
    data: [u8; 32],
}

impl sequential_storage::map::Value<'_> for MyStorageValue {
    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, sequential_storage::map::SerializationError> {
        buffer[0..32].copy_from_slice(&self.data);
        Ok(32)
    }

    fn deserialize_from(buffer: &[u8]) -> Result<Self, sequential_storage::map::SerializationError> {
        let mut data = [0u8; 32];
        data.copy_from_slice(&buffer[0..32]);
        Ok(MyStorageValue { data })
    }
}

// Write
async fn write_custom_data(
    flash: &mut Flash,
    storage_range: Range<usize>,
    buffer: &mut [u8],
    my_data: &[u8; 32],
) -> Result<(), ()> {
    let value = MyStorageValue { data: *my_data };

    store_item(
        flash,
        storage_range,
        &mut NoCache::new(),
        buffer,
        &MY_CUSTOM_KEY,
        &value,
    )
    .await
    .map_err(|_| ())
}

// Read
async fn read_custom_data(
    flash: &mut Flash,
    storage_range: Range<usize>,
    buffer: &mut [u8],
) -> Result<Option<[u8; 32]>, ()> {
    let result = fetch_item::<u32, MyStorageValue, _>(
        flash,
        storage_range,
        &mut NoCache::new(),
        buffer,
        &MY_CUSTOM_KEY,
    )
    .await
    .map_err(|_| ())?;

    Ok(result.map(|v| v.data))
}
```

**Pros**: No RMK fork needed, simpler integration
**Cons**: Not integrated with RMK's storage task, you manage flash access directly

## Important Considerations

### Buffer Size
- Default buffer is 256 bytes minimum (aligned to 32 bytes)
- Macro storage can be up to `MACRO_SPACE_SIZE` (default 512 bytes)
- Ensure your serialized data fits in buffer

### Flash Address
- Your `start_addr: 0xA0000` must not overlap with:
  - Application code (typically up to ~0x70000-0x80000)
  - Softdevice (if using BLE stack separately)
  - Bootloader (if present)
- Check your `memory.x` linker script for safe ranges

### Storage Size
- 12 sectors ◊ 4KB = 48KB is generous for keyboard config
- RMK typically needs ~8-16KB for full keymap + macros + bonds
- Monitor actual usage with debug logs

### Wear Leveling
- Sequential-storage automatically minimizes erases
- Frequent writes append until sector full
- Compaction happens automatically
- Each nRF52840 sector: ~10,000 erase cycles minimum

### Error Handling
- Storage operations can fail (corruption, full, etc.)
- Always check return values
- Use `print_storage_error()` for debugging
- Consider factory reset mechanism (`clear_storage: true`)

## Debugging Storage

### Enable Storage Logs

RMK uses `defmt` for logging. Set log level in `Cargo.toml`:

```toml
[dependencies]
defmt = "0.3"

[profile.dev.package.rmk]
opt-level = "z"  # Don't strip logs

[profile.release.package.rmk]
opt-level = "z"
```

### Check Storage State

The `FLASH_CHANNEL` communication uses signals that you can monitor:

```rust
use rmk::channel::FLASH_OPERATION_FINISHED;

// Wait for any storage operation
FLASH_OPERATION_FINISHED.wait().await;
info!("Storage operation completed");
```

### Clear Storage

To reset storage completely:

```rust
// In your StorageConfig
let storage_config = StorageConfig {
    start_addr: 0xA0000,
    num_sectors: 12,
    clear_storage: true,  // ê Set to true, flash once, then set back to false
    clear_layout: false,
};
```

### Inspect Flash Manually

Using probe-rs or JLink:

```bash
# Read flash region (hex dump)
probe-rs read --chip nRF52840_xxAA 0xA0000 0x1000

# Erase specific sectors
probe-rs erase --chip nRF52840_xxAA 0xA0000 0xC000
```

## Summary

### How Storage Works
1. **Init**: `initialize_encoder_keymap_and_storage()` sets up flash range and loads saved config
2. **Runtime**: Storage task processes messages from `FLASH_CHANNEL`
3. **Persistence**: `sequential-storage` manages append-only sectors with automatic compaction
4. **Serialization**: All data serialized with type-identifier byte + BigEndian encoding

### To Add Custom Data
1. Choose unused storage key
2. Define data structure
3. Implement `Value` trait (serialize/deserialize)
4. Add message type to `FlashOperationMessage`
5. Handle in storage task
6. Use `FLASH_CHANNEL.send()` to write

### Key Files
- **Your config**: [src/main.rs:185-190](../../../src/main.rs#L185-L190)
- **Storage setup**: [src/main.rs:212](../../../src/main.rs#L212)
- **RMK storage module**: `~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/storage/mod.rs`
- **RMK config types**: `~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/config/mod.rs`
- **Channel definitions**: `~/.cargo/git/checkouts/rmk-*/f0872fa/rmk/src/channel.rs`

### Resources
- **sequential-storage docs**: https://docs.rs/sequential-storage/
- **embedded-storage traits**: https://docs.rs/embedded-storage/
- **RMK GitHub**: https://github.com/HaoboGu/rmk
