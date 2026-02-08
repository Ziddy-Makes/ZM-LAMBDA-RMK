use rmk::keyboard_macros::{define_macro_sequences, to_macro_sequence};
use rmk::morse::Morse;
use rmk::types::action::{Action, EncoderAction, KeyAction, KeyboardAction, MorseMode, MorseProfile};
use rmk::types::modifier::ModifierCombination;
use rmk::{a, encoder, k, layer, lt, td};

// Modifier combination aliases
const _LCTRL: ModifierCombination = ModifierCombination::LCTRL;
const _CTRL_ALT: ModifierCombination = ModifierCombination::new()
    .with_left_ctrl(true)
    .with_left_alt(true);
const _CTRL_SHIFT_GUI: ModifierCombination = ModifierCombination::new()
    .with_left_ctrl(true)
    .with_left_shift(true)
    .with_left_gui(true);

// BLE profile actions - User(0-2) for BLE1-3, User(5) for clear, User(6) for USB/BLE switch, User(7) for battery check
const BLE1: Action = Action::User(0);
const BLE2: Action = Action::User(1);
const BLE3: Action = Action::User(2);
const BLE_CLR: Action = Action::User(5);
const USB_BLE_SW: Action = Action::User(6);
const BATT_CHECK: Action = Action::User(7);

pub(crate) const COL: usize = 4;
pub(crate) const ROW: usize = 4;
pub(crate) const SIZE: usize = 16; // Rows * Cols
pub(crate) const NUM_LAYER: usize = 8;
pub(crate) const NUM_ENCODER: usize = 1;

#[rustfmt::skip]
pub const fn get_default_keymap() -> [[[KeyAction; COL]; ROW]; NUM_LAYER] {
    [
        layer!([
            [k!(A),                    k!(B),                      k!(C),                  lt!(1, AudioMute)],
            [k!(D),                    k!(E),                      k!(F),                  k!(G)],
            [k!(H),                    k!(I),                      k!(J),                  k!(K)],
            [k!(L),                    a!(No),                     k!(N),                  k!(O)]
        ]),
        layer!([
            [KeyAction::Single(BLE1),  KeyAction::Single(BLE2),    KeyAction::Single(BLE3),   a!(Transparent)],
            [td!(0),                   a!(No),                     a!(No),                    KeyAction::Single(BATT_CHECK)],
            [td!(1),                   a!(No),                     a!(No),                    KeyAction::Single(USB_BLE_SW)],
            [a!(No),                   a!(No),                     a!(No),                    a!(No)]
        ]),
        layer!([
            [k!(J),                    k!(K),                      k!(L),                  a!(No)],
            [k!(M),                    k!(N),                      k!(O),                  a!(No)],
            [k!(P),                    k!(Q),                      k!(R),                  a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)]
        ]),
        layer!([
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)]
        ]),
        layer!([
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)]
        ]),
        layer!([
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)]
        ]),
        layer!([
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)]
        ]),
        layer!([
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)],
            [a!(No),                   a!(No),                     a!(No),                 a!(No)]
        ]),
    ]
}

pub const fn get_default_encoder_map() -> [[EncoderAction; NUM_ENCODER]; NUM_LAYER] {
    [
        [encoder!(k!(AudioVolUp), k!(AudioVolDown))],
        [encoder!(k!(No), k!(No))],
        [encoder!(k!(No), k!(No))],
        [encoder!(k!(No), k!(No))],
        [encoder!(k!(No), k!(No))],
        [encoder!(k!(No), k!(No))],
        [encoder!(k!(No), k!(No))],
        [encoder!(k!(No), k!(No))],
    ]
}

/// Configure tapdance behaviors
/// This function sets up tapdance configurations that can be referenced in the keymap using td!(index)
pub fn configure_tapdance(behavior_config: &mut rmk::config::BehaviorConfig) {
    use rmk::morse::{HOLD, TAP};

    // Tapdance 0 - Hold for BLE clear
    let mut td0 = Morse::default();
    td0.profile = MorseProfile::new(
        None,                    // Use default unilateral_tap
        Some(MorseMode::Normal), // Normal mode
        Some(200),               // 200ms hold timeout
        Some(200),               // 200ms gap timeout
    );
    td0.put(HOLD, BLE_CLR);

    //////////////////////////////////////////////////////////////////////////////

    // Tapdance 1 - Hold for bootloader
    let mut td1 = Morse::default();
    td1.profile = MorseProfile::new(None, Some(MorseMode::Normal), Some(200), Some(200));
    td1.put(HOLD, Action::KeyboardControl(KeyboardAction::Bootloader));

    //////////////////////////////////////////////////////////////////////////////

    // Tapdance 2 - Tap for BLE3, Hold for BLE clear
    let mut td2 = Morse::default();
    td2.profile = MorseProfile::new(None, Some(MorseMode::Normal), Some(200), Some(200));
    td2.put(TAP, BLE3);  // User(2) = BLE3
    td2.put(HOLD, BLE_CLR);  // User(5) = BLE_CLR

    //////////////////////////////////////////////////////////////////////////////

    // Add tapdance configurations to behavior_config
    let _ = behavior_config.morse.morses.push(td0);
    let _ = behavior_config.morse.morses.push(td1);
    let _ = behavior_config.morse.morses.push(td2);
}

/// Configure keyboard macros
/// This function sets up macro sequences that can be triggered using Action::TriggerMacro(index)
pub fn configure_macros(behavior_config: &mut rmk::config::BehaviorConfig) {
    // Use in Keymap array
    // KeyAction::Single(Action::TriggerMacro(0))

    // Macro 0: Text macro example
    let macro0 = to_macro_sequence("Ziddy Makes was here (:");

    // Create macro sequences array and define them
    let macro_sequences = [macro0];
    let binary_macros = define_macro_sequences(&macro_sequences);
    behavior_config.keyboard_macros.macro_sequences = binary_macros;
}
