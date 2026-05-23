use defmt::info;
use embassy_nrf::gpio::Output;
use embassy_nrf::spim::Spim;
use rmk::event::{
    ActionEvent, BatteryStatusEvent, ChargingStateEvent, ConnectionStatusChangeEvent,
    ConnectionType,
};
use rmk::macros::processor;
use rmk::types::action::Action;
use rmk::types::battery::BatteryStatus;
use rmk::types::ble::BleState;
use smart_leds::{RGB8, SmartLedsWrite};
use ws2812_spi::Ws2812;

#[processor(
    subscribe = [
        ConnectionStatusChangeEvent,
        BatteryStatusEvent,
        ChargingStateEvent,
        ActionEvent,
    ],
    poll_interval = 1400,
)]
pub struct StatusLedController<'d, const N: usize> {
    ws2812: Ws2812<Spim<'d>>,
    power_pin: Output<'d>,
    should_blink: bool,
    leds_on: bool,
    current_ble_profile: u8,
    last_ble_state: BleState,
    active_transport: Option<ConnectionType>,
    battery_percentage: u8,
    is_showing_battery: bool,
    user7_held: bool,
    charging: bool,
}

impl<'d, const N: usize> StatusLedController<'d, N> {
    pub fn new(ws2812: Ws2812<Spim<'d>>, power_pin: Output<'d>) -> Self {
        Self {
            ws2812,
            power_pin,
            should_blink: true, // Start true - we're advertising on boot, event may be missed due to race
            leds_on: false,
            current_ble_profile: 0,
            last_ble_state: BleState::Inactive,
            active_transport: None,
            battery_percentage: 100,
            is_showing_battery: false,
            user7_held: false,
            charging: false,
        }
    }

    fn blink_ble_profile_led_blue(&mut self) {
        self.power_pin.set_high();
        info!(
            "Blinking blue LED: {} (max: {})",
            self.current_ble_profile, N
        );
        let mut data = [RGB8 { r: 0, g: 0, b: 0 }; N];

        let profile_index = (self.current_ble_profile as usize).min(N - 1);
        data[profile_index] = RGB8 { r: 0, g: 0, b: 70 };

        match self.ws2812.write(data.iter().cloned()) {
            Ok(_) => {
                info!("Successfully wrote LED data");
                self.leds_on = true;
            }
            Err(_) => {
                info!("Failed to write LED data");
            }
        }
    }

    fn blink_ble_profile_led_green(&mut self) {
        self.power_pin.set_high();
        info!(
            "Blinking green LED: {} (max: {})",
            self.current_ble_profile, N
        );
        let mut data = [RGB8 { r: 0, g: 0, b: 0 }; N];

        let profile_index = (self.current_ble_profile as usize).min(N - 1);
        data[profile_index] = RGB8 { r: 0, g: 70, b: 0 };

        match self.ws2812.write(data.iter().cloned()) {
            Ok(_) => {
                info!("Successfully wrote LED data");
                self.leds_on = true;
            }
            Err(_) => {
                info!("Failed to write LED data");
            }
        }
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
            1
        } else if self.battery_percentage >= 89 {
            N // 89-100% = all N LEDs
        } else {
            // 1-88% maps to 1-(N-1) LEDs: scale proportionally
            ((self.battery_percentage as usize - 1) * (N - 1) / 88) + 1
        };

        // Charging gets a distinct amber tint; otherwise red < 30% else green.
        let led_color = if self.charging {
            RGB8 { r: 70, g: 35, b: 0 } // Amber while charging
        } else if self.battery_percentage < 30 {
            RGB8 { r: 70, g: 0, b: 0 } // Red for low battery
        } else {
            RGB8 { r: 0, g: 70, b: 0 } // Green for normal battery
        };

        let mut data = [RGB8::default(); N];
        for i in 0..num_leds {
            data[i] = led_color;
        }

        let _ = self.ws2812.write(data.iter().cloned());
        self.leds_on = true;

        info!(
            "Battery level: {}% ({} LEDs, charging={})",
            self.battery_percentage, num_leds, self.charging
        );
    }

    // Event handlers for #[processor] macro

    async fn on_connection_status_change_event(&mut self, event: ConnectionStatusChangeEvent) {
        let status = event.0;
        let active = status.decide_active();
        let prev_transport = self.active_transport;
        self.active_transport = active;

        let prev_ble_state = self.last_ble_state;
        self.last_ble_state = status.ble.state;
        self.current_ble_profile = status.ble.profile;

        info!(
            "ConnectionStatus changed: active={:?}, ble.state={:?}, ble.profile={}",
            active, status.ble.state, status.ble.profile
        );

        // Transport-routing branch (replaces old on_connection_change_event):
        // Once USB is active, kill BLE advertising indicators.
        if matches!(active, Some(ConnectionType::Usb))
            && !matches!(prev_transport, Some(ConnectionType::Usb))
        {
            info!("USB active - stopping BLE indicators");
            self.should_blink = false;
            if !self.is_showing_battery {
                self.clear_all_leds();
            }
        }

        // BLE state branch (replaces old on_ble_status_change_event):
        match status.ble.state {
            BleState::Advertising => {
                // Don't override USB-active suppression: only blink when BLE is the
                // intended path (no USB active).
                if !matches!(active, Some(ConnectionType::Usb)) {
                    info!(
                        "Advertising - profile: {}",
                        status.ble.profile
                    );
                    self.should_blink = true;
                }
            }
            BleState::Connected => {
                self.should_blink = false;
                // Only flash green on a real transition into Connected so it doesn't
                // re-fire on unrelated status changes (e.g. USB suspend toggles).
                if prev_ble_state != BleState::Connected {
                    info!("Connected - profile: {}", status.ble.profile);
                    for _ in 0..4 {
                        self.blink_ble_profile_led_green();
                        embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
                        self.clear_all_leds();
                        embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
                    }
                }
            }
            BleState::Inactive => {
                self.should_blink = false;
                if !self.is_showing_battery {
                    info!("Inactive");
                    self.clear_all_leds();
                }
            }
        }
    }

    async fn on_battery_status_event(&mut self, event: BatteryStatusEvent) {
        match event.0 {
            BatteryStatus::Unavailable => {
                info!("Battery not available");
            }
            BatteryStatus::Available {
                charge_state: _,
                level,
            } => match level {
                Some(pct) => {
                    self.battery_percentage = pct;
                    info!("Battery updated: {}%", pct);
                }
                None => info!("Battery level unknown"),
            },
        }
    }

    async fn on_charging_state_event(&mut self, event: ChargingStateEvent) {
        self.charging = event.charging;
        info!("Charging state changed: {}", self.charging);
        // Refresh battery LED if currently visible so the colour reflects the new state.
        if self.is_showing_battery {
            self.show_battery_level();
        }
    }

    async fn on_action_event(&mut self, event: ActionEvent) {
        // Check if it's User7 action (BAT_CHK in Vial)
        if let Action::User(7) = event.action {
            if !self.user7_held {
                info!("User7 (BAT_CHK) pressed - showing battery level");
                self.user7_held = true;
                self.is_showing_battery = true;
                self.show_battery_level();
            } else {
                info!("User7 (BAT_CHK) released - clearing battery display");
                self.user7_held = false;
                self.is_showing_battery = false;
                self.clear_all_leds();
            }
        }
    }

    /// Called by PollingProcessor::update() every 1400ms (poll_interval)
    async fn poll(&mut self) {
        // Only blink for BLE if we're not currently showing battery level
        if self.should_blink && !self.is_showing_battery {
            info!(
                "Blinking: leds_on={}, profile={}",
                self.leds_on, self.current_ble_profile
            );
            if self.leds_on {
                self.clear_all_leds();
            } else {
                self.blink_ble_profile_led_blue();
            }
        }
    }
}
