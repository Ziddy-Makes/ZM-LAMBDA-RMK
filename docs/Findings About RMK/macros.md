# Macros

## Memory Usage

Macros in RMK take up RAM, specifically **2048 bytes** (2 KB).

The main memory usage comes from:

1. **`macro0` vector**: `Vec::<MacroOperation, 2048>` - This allocates a heapless vector with a capacity of 2048 bytes on the stack while building the macro.

2. **`behavior_config.keyboard_macros.macro_sequences`**: This is a `[u8; 2048]` array that stores the compiled binary macro data. This is the persistent storage that stays in RAM throughout runtime.

### Example

A macro that holds Ctrl+Shift+Alt+Gui, taps 4, and releases all modifiers only uses about **27 bytes** of the 2048-byte buffer:
- 9 operations × 3 bytes each (Press/Release operations) = 27 bytes
- Plus 1 byte for the End marker = 28 bytes total

The rest of the 2048 bytes are filled with zeros but still consume RAM. This size is fixed by RMK's `MACRO_SPACE_SIZE` constant to allow room for multiple macros and for Vial to dynamically add macros at runtime.

If RAM is tight, you could potentially reduce memory usage by modifying RMK's configuration, but that would require changing the build configuration and might limit your ability to use Vial's dynamic macro feature.

## Storage and Flash Memory

Macros are **both stored in RAM and persisted to flash memory**:

### Initial Loading
1. At boot, RMK initializes macros from your code (defined in `configure_macros()`)
2. RMK then reads any saved macro data from flash storage using `read_macro_cache()`
3. If macros exist in flash, they **overwrite** the default macros from code
4. The macro data is loaded into the RAM buffer (`behavior_config.keyboard_macros.macro_sequences`)

### Saving to Flash
Macros are saved to flash when:
- You modify macros through **Vial** (the dynamic keymap editor)
- Vial sends a macro update command, which triggers `FlashOperationMessage::WriteMacro()`
- The entire 2048-byte macro buffer is written to flash storage

### Key Points
- **Flash storage key**: `StorageKeys::MacroData` (key ID: 6)
- **Flash writes**: Only happen when macros are modified via Vial, not on every keypress
- **Persistence**: Macros defined in code are the default, but any Vial changes persist across reboots
- **Flash location**: Stored in the flash region defined by your `StorageConfig` (in your case: starting at `0xA0000`, using 12 sectors = 48KB total)

### What This Means
- Macros you define in `configure_macros()` are **default values**
- Once you edit macros in Vial, those changes are saved to flash and will override your code defaults
- To reset macros to code defaults, you need to clear storage or flash new firmware

## Potential Optimization: Streaming from Flash

### Current Limitation
The current RMK implementation loads **all** macro data into a 2048-byte RAM buffer at boot. This has significant limitations:

**Problems:**
- **RAM consumption**: 2KB always allocated, even if you only use small macros
- **Size constraints**: Total of ALL macros combined cannot exceed 2048 bytes
- **Long text macros**: Cannot store macros with more than ~680 characters (3 bytes per character operation)
- **Multiple macros**: The 2048-byte limit is shared across all macros (Macro0, Macro1, etc.)

### Why Streaming Would Be Better

**Flash vs RAM availability on nRF52840:**
- **Flash**: 1024 KB (1 MB)
- **RAM**: 256 KB
- **Current storage allocation**: 48 KB of flash (12 sectors × 4KB each)

**Benefits of streaming from flash:**
1. **Massive capacity**: Could store hundreds of KB of macro data instead of 2KB
2. **RAM savings**: Only need a small buffer (~32-64 bytes) to read one operation at a time
3. **Long macros**: Could easily store entire paragraphs or code snippets
4. **More macros**: Could support dozens of macros instead of being limited by total size

### How It Would Work

Instead of loading all macros into RAM:
```rust
// Current: All macros in RAM
behavior_config.keyboard_macros.macro_sequences = [u8; 2048];
```

Streaming approach would:
```rust
// 1. Store flash offset/address of each macro
macro_locations: [(flash_addr, length); 32]

// 2. During execution, read operations one at a time from flash
loop {
    let operation = flash.read_bytes(current_flash_addr, 3); // Read one operation
    execute(operation);
    current_flash_addr += operation_size;
}
```

### Current Implementation Details

Looking at the code in `keyboard.rs:execute_macro()`:
- It reads the entire macro buffer from RAM: `self.keymap.borrow().behavior.keyboard_macros.macro_sequences`
- Uses `get_next_macro_operation()` to parse operations sequentially
- **Already processes macros one operation at a time** with delays between operations
- This sequential processing pattern is **perfect** for streaming from flash

### Why RMK Doesn't Do This (Yet)

Possible reasons:
1. **Simplicity**: Loading everything into RAM is simpler to implement
2. **Vial compatibility**: The current design matches QMK/Vial's expectations for macro storage format
3. **Flash wear**: Flash has limited write cycles; streaming would require careful caching
4. **Performance**: Flash reads are slower than RAM (though likely negligible for human-speed macro playback)
5. **Development priority**: Most users don't need massive macros, so it hasn't been prioritized

### Feasibility

This optimization is **technically feasible** because:
- ✅ RMK already has flash read/write infrastructure (`Storage` module)
- ✅ Macros are already processed sequentially, one operation at a time
- ✅ There are natural delays (1-12ms) between operations where flash reads could happen
- ✅ The nRF52840 has plenty of flash available

### Workaround for Now

If you need very long macros or many macros:
1. **Split large macros** into multiple smaller macros and trigger them sequentially
2. **Use User keycodes** with custom handlers in your code instead of the macro system
3. **Store text in code** and type it programmatically rather than using macro sequences
4. **Consider a feature request** to the RMK project for flash-streaming macro support

### Recommendation

This would be an **excellent enhancement** to propose to the RMK project. Given that:
- RAM is precious on microcontrollers (only 256 KB total)
- Flash is abundant (1 MB, with only 48 KB used for storage currently)
- The infrastructure is already in place
- The sequential processing pattern is perfect for streaming

A hybrid approach might work best:
- **Small macros** (< 128 bytes): Keep in RAM for speed
- **Large macros** (> 128 bytes): Stream from flash on-demand

## How QMK/Vial Handles This

### QMK/Vial **STREAMS** from EEPROM/Flash

After investigating QMK's source code, **QMK/Vial does NOT load macros into RAM** - it streams them directly from EEPROM during execution!

**Implementation in QMK (`quantum/dynamic_keymap.c`):**

```c
void dynamic_keymap_macro_send(uint8_t id) {
    // Find macro by counting null terminators in EEPROM
    // Then read bytes one at a time:
    uint8_t data = eeprom_read_byte(p++);  // Read one byte from EEPROM
    // Process the byte
    // Repeat until null terminator
}
```

**How it works:**
1. Macros stored in EEPROM at `DYNAMIC_KEYMAP_MACRO_EEPROM_ADDR`
2. During execution, `eeprom_read_byte(p++)` reads **one byte at a time**
3. Processes each operation immediately (tap, press, release, delay)
4. No RAM buffer needed except for tiny temporary variables
5. Can handle macros up to **65,535 bytes** (entire EEPROM size limit)

### Key Differences from RMK

| Aspect | QMK/Vial | RMK (Current) |
|--------|----------|---------------|
| **Loading** | Streams from EEPROM | Loads all into RAM (2048 bytes) |
| **RAM usage** | ~10-20 bytes (variables only) | 2048 bytes constant |
| **Max macro size** | Up to 65KB (EEPROM limit) | 2048 bytes total for ALL macros |
| **Execution** | Read byte → Execute → Read next | Parse from RAM buffer |
| **Storage** | EEPROM (emulated in flash) | Flash → RAM on boot |

### Why QMK Can Do This

1. **EEPROM abstraction**: QMK uses `eeprom_read_byte()` which works whether using real EEPROM or flash-emulated EEPROM
2. **Simple sequential reads**: Macros are stored as a linear byte stream, perfect for streaming
3. **Natural delays**: Macro playback has 1-12ms delays between operations, plenty of time for flash reads
4. **No Vial editing during playback**: Macros aren't modified while executing, so no race conditions

### Why This Matters for RMK

**RMK could adopt the exact same approach:**
- Replace the 2048-byte RAM buffer with streaming reads from flash
- Use the existing `Storage` infrastructure to read bytes on-demand
- Would free up **2KB of RAM** (that's ~0.8% of total RAM!)
- Could support much larger macros (limited only by flash storage space)

### The Answer to Your Question

**Yes, QMK/Vial already does exactly what you suggested!** They stream macros from flash/EEPROM rather than loading everything into RAM. This is proof that the approach works perfectly in production keyboard firmware and would be an excellent enhancement for RMK to adopt.

## Critical Issue: Stack Overflow with Large `macro_space_size`

### The Problem

**Setting `macro_space_size` above 2048 in `keyboard.toml` causes runtime crashes**, even though it compiles successfully.

### Root Cause: Stack Allocation

The crash happens because the `configure_macros()` function allocates large buffers **on the stack**:

```rust
pub fn configure_macros(behavior_config: &mut rmk::config::BehaviorConfig) {
    // This allocates MACRO_SPACE_SIZE bytes on the STACK!
    let mut macro0 = Vec::<MacroOperation, MACRO_SPACE_SIZE>::new();

    // Build macro...

    // define_macro_sequences also uses stack internally
    let binary_macros = define_macro_sequences(&macro_sequences);
}
```

### Why It Crashes

1. **Stack is limited**: Cortex-M microcontrollers typically have small stacks (8-16KB)
2. **Function call allocates on stack**: When `configure_macros()` is called during boot:
   - `Vec::<MacroOperation, 2048>` = 2048 bytes on stack
   - `define_macro_sequences()` internal buffers = additional 2048+ bytes
   - Other function call overhead
   - **Total: ~4-6KB of stack usage** just for this function
3. **With macro_space_size = 4096**: Would need ~8-12KB stack just for this function
4. **Stack overflow**: Exceeds available stack → **crash**

### Why It Compiles But Crashes

- **Compile time**: The compiler can't predict stack usage across all function calls
- **Runtime**: When the function actually executes, it tries to allocate more stack than available
- **Result**: Silent stack overflow → undefined behavior → crash

### Memory Layout

The nRF52840 has:
- **RAM**: 256 KB total
- **Stack**: Grows downward from top of RAM
- **Heap/Static**: Grows upward from bottom of RAM
- **No stack size checks**: Stack overflow silently corrupts other memory

### Solutions

#### Option 1: Don't Use Large Macros (Current)
Keep `macro_space_size = 2048` or smaller.

#### Option 2: Allocate Statically (Workaround)
Move the allocation out of the function to static memory:

```rust
use static_cell::StaticCell;

static MACRO_BUFFER: StaticCell<[u8; 4096]> = StaticCell::new();

pub fn configure_macros(behavior_config: &mut rmk::config::BehaviorConfig) {
    // Use static allocation instead of stack
    let macro_buf = MACRO_BUFFER.init([0u8; 4096]);
    // ... build macros into macro_buf
}
```

#### Option 3: Increase Stack Size (Risky)
Modify linker script to allocate more stack, but this reduces heap/static memory available.

#### Option 4: Use Box/Allocator (Not Available)
Embedded systems typically don't have heap allocators enabled in no_std environments.

### The Real Solution: Stream from Flash

This is **yet another reason** why RMK should adopt QMK's streaming approach:
- ❌ Current: Needs 2-4KB of stack during initialization
- ✅ Streaming: Would need <100 bytes of stack
- ✅ Would eliminate this crash completely
- ✅ Would support arbitrarily large macros

### Current Limitations Summary

| Limit | Value | Reason |
|-------|-------|--------|
| `macro_space_size` max | ~2048 bytes | Stack overflow |
| Total macro RAM | 2048 bytes | Const in code |
| Stack usage during init | ~4-6 KB | Temporary buffers |
| Max safe increase | ~3072 bytes | Stack size dependent |

### Recommendation

**Do not increase `macro_space_size` above 2048 bytes** until RMK implements flash streaming. The stack overflow risk is too high and will cause hard-to-debug crashes.
