use defmt::{info, unwrap};
use embassy_nrf::gpio::Output;
use embassy_nrf::spim::Spim;
use rmk::ble::BleState;
use rmk::channel::CONTROLLER_CHANNEL;
use rmk::channel::ControllerSub;
use rmk::controller::{Controller, PollingController};
use rmk::event::ControllerEvent;
use rmk::types::action::{Action, KeyAction};
use rmk::types::keycode::KeyCode;
use smart_leds::{RGB8, SmartLedsWrite};
use ws2812_spi::Ws2812;

pub struct StatusLedController<'d, const N: usize> {
    ws2812: Ws2812<Spim<'d>>,
    power_pin: Output<'d>,
    sub: ControllerSub,
    should_blink: bool,
    leds_on: bool,
    current_ble_profile: u8,
    battery_percentage: u8,
    is_showing_battery: bool,
    user7_held: bool,
}

impl<'d, const N: usize> StatusLedController<'d, N> {
    pub fn new(ws2812: Ws2812<Spim<'d>>, power_pin: Output<'d>) -> Self {
        Self {
            ws2812,
            power_pin,
            sub: unwrap!(CONTROLLER_CHANNEL.subscriber()),
            should_blink: false,
            leds_on: false,
            current_ble_profile: 0,
            battery_percentage: 100,
            is_showing_battery: false,
            user7_held: false,
        }
    }

    fn blink_ble_profile_led_blue(&mut self) {
        self.power_pin.set_high();
        info!("Blinking blue LED: {}", self.current_ble_profile);
        let mut data = [RGB8 { r: 0, g: 0, b: 0 }; N];
        data[self.current_ble_profile as usize] = RGB8 { r: 0, g: 0, b: 70 };
        let _ = self.ws2812.write(data.iter().cloned());
        self.leds_on = true;
    }

    fn blink_ble_profile_led_green(&mut self) {
        self.power_pin.set_high();
        let mut data = [RGB8 { r: 0, g: 0, b: 0 }; N];
        data[self.current_ble_profile as usize] = RGB8 { r: 0, g: 70, b: 0 };
        let _ = self.ws2812.write(data.iter().cloned());
        self.leds_on = true;
    }

    fn clear_all_leds(&mut self) {
        let data = [RGB8::default(); N];
        let _ = self.ws2812.write(data.iter().cloned());
        self.power_pin.set_low();
        self.leds_on = false;
    }

    fn show_battery_level(&mut self) {
        self.power_pin.set_high();

        // Calculate how many LEDs to light up based on battery percentage
        // Map 0-100% to 0-N LEDs (with at least 1 LED if battery > 0%)
        let num_leds = if self.battery_percentage == 0 {
            //0
            N
        } else if self.battery_percentage >= 89 {
            N // 89-100% = all N LEDs
        } else {
            // 1-88% maps to 1-(N-1) LEDs: scale proportionally
            ((self.battery_percentage as usize - 1) * (N - 1) / 88) + 1
        };

        // Choose color based on battery level: red if under 30%, green otherwise
        let led_color = if self.battery_percentage < 30 {
            RGB8 { r: 70, g: 0, b: 0 } // Red for low battery
        } else {
            RGB8 { r: 0, g: 70, b: 0 } // Green for normal battery
        };

        // Create LED array and light up the first num_leds
        let mut data = [RGB8::default(); N];
        for i in 0..num_leds {
            data[i] = led_color;
        }

        let _ = self.ws2812.write(data.iter().cloned());
        self.leds_on = true;

        info!(
            "Battery level: {}% ({} LEDs, {})",
            self.battery_percentage,
            num_leds,
            if self.battery_percentage < 30 {
                "RED"
            } else {
                "GREEN"
            }
        );
    }
}

impl<'d, const N: usize> Controller for StatusLedController<'d, N> {
    type Event = ControllerEvent;

    async fn process_event(&mut self, event: Self::Event) {
        match event {
            ControllerEvent::BleState(profile, state) => {
                match state {
                    BleState::Advertising => {
                        // Start blinking blue when advertising
                        info!("Advertising - Custom Controller - Profile: {}", profile);
                        self.current_ble_profile = profile;
                        self.should_blink = true;
                    }
                    BleState::Connected => {
                        // Stop blinking and blink green 3 times
                        self.should_blink = false;
                        self.current_ble_profile = profile;
                        info!("Connected - Custom Controller - Profile: {}", profile);

                        // Blink green 3 times
                        for _ in 0..4 {
                            self.blink_ble_profile_led_green();
                            embassy_time::Timer::after(embassy_time::Duration::from_millis(500))
                                .await;
                            self.clear_all_leds();
                            embassy_time::Timer::after(embassy_time::Duration::from_millis(500))
                                .await;
                        }
                    }
                    BleState::None => {
                        // Turn off LEDs when not in BLE mode
                        self.should_blink = false;
                        info!("None - Custom Controller");
                        self.clear_all_leds();
                    }
                }
            }
            ControllerEvent::Battery(percentage) => {
                // Update battery percentage when received from BatteryProcessor
                self.battery_percentage = percentage;
                info!("Battery updated: {}%", percentage);
            }
            ControllerEvent::Key(_keyboard_event, key_action) => {
                // Check if it's User7 key (BAT_CHK in Vial)
                if let KeyAction::Single(Action::Key(KeyCode::User7)) = key_action {
                    // Toggle the state - if not currently held, it's a press; otherwise it's a release
                    if !self.user7_held {
                        // User7 pressed - show battery level
                        info!("User7 (BAT_CHK) pressed - showing battery level");
                        self.user7_held = true;
                        self.is_showing_battery = true;
                        self.show_battery_level();
                    } else {
                        // User7 released - clear LEDs
                        info!("User7 (BAT_CHK) released - clearing battery display");
                        self.user7_held = false;
                        self.is_showing_battery = false;
                        self.clear_all_leds();
                    }
                }
            }
            _ => {
                // Ignore other events
            }
        }
    }

    async fn next_message(&mut self) -> Self::Event {
        self.sub.next_message_pure().await
    }
}

impl<'d, const N: usize> PollingController for StatusLedController<'d, N> {
    const INTERVAL: embassy_time::Duration = embassy_time::Duration::from_millis(700);

    async fn update(&mut self) {
        // Only blink for BLE if we're not currently showing battery level
        if self.should_blink && !self.is_showing_battery {
            if self.leds_on {
                self.clear_all_leds();
            } else {
                // self.set_all_leds_blue();
                self.blink_ble_profile_led_blue();
            }
        }
    }
}
