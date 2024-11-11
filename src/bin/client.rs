//% FEATURES: esp-wifi esp-wifi/wifi-default esp-wifi/wifi esp-wifi/utils esp-wifi/esp-now
//% CHIPS: esp32 esp32s2 esp32s3 esp32c2 esp32c3 esp32c6

#![no_std]
#![no_main]

use adafruit_7segment::{Index, SevenSegment};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Io, Level, Output},
    i2c::I2c,
    peripherals::I2C0,
    prelude::*,
    rng::Rng,
    time::{self, Duration},
    timer::timg::TimerGroup,
    Blocking,
};
use esp_println::println;
use esp_wifi::{
    esp_now::{PeerInfo, BROADCAST_ADDRESS},
    init, EspWifiInitFor,
};
use ht16k33::{Dimming, Display, LedLocation, HT16K33};

use tactile_tesla::NowMessage;

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    esp_alloc::heap_allocator!(72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    let init = init(
        EspWifiInitFor::Wifi,
        timg0.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let esp_now = esp_wifi::esp_now::EspNow::new(&init, wifi).unwrap();

    println!("esp-now version {}", esp_now.get_version().unwrap());

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let i2c = I2c::new(peripherals.I2C0, io.pins.gpio27, io.pins.gpio25, 100.kHz());
    let mut display: HT16K33<I2c<'_, I2C0, Blocking>> = HT16K33::new(i2c, 0x70);

    display.initialize().expect("Failed to initialize ht16k33");
    display
        .set_display(Display::ON)
        .expect("Could not turn on the display!");
    display
        .set_dimming(Dimming::BRIGHTNESS_MAX)
        .expect("Could not set dimming!");
    display.update_buffer_with_dot(Index::Four, true);
    display.write_display_buffer().ok();

    loop {
        let r = esp_now.receive();
        if let Some(r) = r {
            println!("Got message. Decoding...");
            match bincode::decode_from_slice::<NowMessage, _>(&r.data, bincode::config::standard())
            {
                Ok((message, _len)) => {
                    println!("Got message: {message:?}");
                    match message {
                        NowMessage::Speedometer { speed } => {
                            write_number(&mut display, speed);
                        }
                    }
                }
                Err(_err) => {}
            }
        }
    }
}

fn write_number(display: &mut HT16K33<I2c<'_, I2C0, Blocking>>, num: u16) {
    if num > 9999 {
        return;
    }

    let i1 = num / 1000;
    let i2 = num % 1000 / 100;
    let i3 = num % 100 / 10;
    let i4 = num % 10;
    display.update_buffer_with_digit(Index::One, i1 as _);
    display.update_buffer_with_digit(Index::Two, i2 as _);
    display.update_buffer_with_digit(Index::Three, i3 as _);
    display.update_buffer_with_digit(Index::Four, i4 as _);
    display.write_display_buffer().ok();
}
