// macro_rules! config_matrix_pins_nrf {
//     (peripherals: $p:ident, input: [$($in_pin:ident), *], output: [$($out_pin:ident), +]) => {
//         {
//             let mut output_pins = [$(Output::new($p.$out_pin, embassy_nrf::gpio::Level::Low, embassy_nrf::gpio::OutputDrive::Standard)), +];
//             let input_pins = [$(Input::new($p.$in_pin, embassy_nrf::gpio::Pull::Down)), +];
//             output_pins.iter_mut().for_each(|p| {
//                 p.set_low();
//             });
//             (input_pins, output_pins)
//         }
//     };
// }



macro_rules! config_matrix_pins_nrf {
    (peripherals: $p:ident, direct_pins: [$([$($pin:tt),+ $(,)?]),+ $(,)?]) => {
        {
            #[allow(unused_mut)]
            let mut pins = [
                $(
                    [
                        $(
                            config_matrix_pin_nrf!(@pin $p, $pin)
                        ),+
                    ]
                ),+
            ];
            pins
        }
    };
}

macro_rules! config_matrix_pin_nrf {
    (@pin $p:ident, _) => {
        None
    };

    (@pin $p:ident, $pin:ident) => {
        Some(Input::new($p.$pin, embassy_nrf::gpio::Pull::Up))
    };
}

/// Create a key action from a KeyCode expression (constant or variant path)
/// Works with constants (like `BLE1`, `BATT_CHECK`) or variant paths (like `KeyCode::User0`)
/// Usage: 
///   - `kc!(BLE1)` for constants
///   - `kc!(KeyCode::A)` for variant names
macro_rules! kc {
    ($k:expr) => {
        rmk::types::action::KeyAction::Single(rmk::types::action::Action::Key($k))
    };
}
