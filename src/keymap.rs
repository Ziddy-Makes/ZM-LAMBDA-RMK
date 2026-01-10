use rmk::keyboard_macros::{define_macro_sequences, to_macro_sequence};
use rmk::morse::Morse;
use rmk::types::action::{Action, EncoderAction, KeyAction, MorseMode, MorseProfile};
use rmk::types::keycode::KeyCode;
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

const BLE1: KeyCode = KeyCode::User0;
const BLE2: KeyCode = KeyCode::User1;
const BLE3: KeyCode = KeyCode::User2;
const BLE_CLR: KeyCode = KeyCode::User5;
const USB_BLE_SW: KeyCode = KeyCode::User6;
const BATT_CHECK: KeyCode = KeyCode::User7;

pub(crate) const COL: usize = 4;
pub(crate) const ROW: usize = 4;
pub(crate) const SIZE: usize = 16; // Rows * Cols
pub(crate) const NUM_LAYER: usize = 8;
pub(crate) const NUM_ENCODER: usize = 1;

#[rustfmt::skip]
pub const fn get_default_keymap() -> [[[KeyAction; COL]; ROW]; NUM_LAYER] {
    [
        layer!([
            [lt!(1, AudioMute),        k!(A),                      k!(B),                  k!(C)],
            [k!(D),                    k!(E),                      k!(F),                  k!(G)],
            [k!(H),                    k!(I),                      k!(J),                  k!(K)],
            [k!(L),                    k!(M),                      k!(N),                  k!(O)]
        ]),
        layer!([
            [kc!(BLE1),                kc!(BLE2),                  kc!(BLE3),                 a!(No)],
            [td!(0),                   a!(No),                     kc!(BATT_CHECK),           a!(No)],
            [td!(1),                   a!(No),                     kc!(USB_BLE_SW),           a!(No)],
            [a!(Transparent),          a!(No),                     a!(No),                    a!(No)]
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
    use rmk::types::keycode::KeyCode;

    // Tapdance 0
    let mut td0 = Morse::default();
    td0.profile = MorseProfile::new(
        None,                    // Use default unilateral_tap
        Some(MorseMode::Normal), // Normal mode
        Some(200),               // 200ms hold timeout
        Some(200),               // 200ms gap timeout
    );
    td0.put(HOLD, Action::Key(BLE_CLR));

    //////////////////////////////////////////////////////////////////////////////

    // Tapdance 1
    let mut td1 = Morse::default();
    td1.profile = MorseProfile::new(None, Some(MorseMode::Normal), Some(200), Some(200));
    td1.put(HOLD, Action::Key(KeyCode::Bootloader));


    //////////////////////////////////////////////////////////////////////////////

    // Tapdance 2
    let mut td2 = Morse::default();
    td2.profile = MorseProfile::new(None, Some(MorseMode::Normal), Some(200), Some(200));
    td2.put(TAP, Action::Key(KeyCode::User2));
    td2.put(HOLD, Action::Key(KeyCode::User5));

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
