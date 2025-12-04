use embassy_nrf::gpio::Output;
use embassy_nrf::spim::Spim;
use embassy_time::Timer;
use smart_leds::{RGB8, SmartLedsWrite};
use ws2812_spi::Ws2812;

pub struct StartupAnimator<'d, const N: usize> {
    ws2812: Ws2812<Spim<'d>>,
    power_pin: Output<'d>,
}

impl<'d, const N: usize> StartupAnimator<'d, N> {
    pub fn new(ws2812: Ws2812<Spim<'d>>, power_pin: Output<'d>) -> Self {
        Self { ws2812, power_pin }
    }

    /// Bootup animation: wave effect from start to end
    pub async fn bootup_animation(&mut self) {
        // Turn on LED power
        self.power_pin.set_high();
        // Wave effect - light up each LED in sequence
        for i in 0..N {
            let mut data = [RGB8::default(); N];
            for j in 0..=i {
                data[j] = RGB8 { r: 60, g: 20, b: 0 }; // Maybe Orange color
            }
            let _ = self.ws2812.write(data.iter().cloned());
            Timer::after_millis(100).await;
        }

        // Flash all LEDs white
        let data = [RGB8 { r: 0, g: 0, b: 50 }; N];
        let _ = self.ws2812.write(data.iter().cloned());
        Timer::after_millis(300).await;

        // Turn off all LEDs
        let data = [RGB8::default(); N];
        let _ = self.ws2812.write(data.iter().cloned());
        Timer::after_millis(50).await;

        // Turn off LED power to save power
        self.power_pin.set_low();
    }

    /// Return the ws2812 controller and power pin for use elsewhere
    pub fn take(self) -> (Ws2812<Spim<'d>>, Output<'d>) {
        (self.ws2812, self.power_pin)
    }
}
