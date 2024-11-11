//! ESP-NOW Example
//!
//! Broadcasts, receives and sends messages via esp-now

//% FEATURES: esp-wifi esp-wifi/wifi-default esp-wifi/wifi esp-wifi/utils esp-wifi/esp-now
//% CHIPS: esp32 esp32s2 esp32s3 esp32c2 esp32c3 esp32c6

#![no_std]
#![no_main]

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, Io, Pull},
    prelude::*,
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_now_poc::NowMessage;
use esp_println::println;
use esp_wifi::{
    esp_now::{EspNow, BROADCAST_ADDRESS},
    init, EspWifiInitFor,
};

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
    let mut esp_now = esp_wifi::esp_now::EspNow::new(&init, wifi).unwrap();

    println!("esp-now version {}", esp_now.get_version().unwrap());

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let button = Input::new(io.pins.gpio0, Pull::Up);

    send_update(&mut esp_now, &NowMessage::Speedometer { speed: 0 });

    let mut speed = 0;
    let mut button_state = false;
    loop {
        let new_button_state = button.is_low();
        if button_state != new_button_state && new_button_state {
            speed += 1;
            send_update(&mut esp_now, &NowMessage::Speedometer { speed })
        }
        button_state = new_button_state;
    }
}

fn send_update(esp_now: &mut EspNow<'_>, message: &NowMessage) {
    for _ in 0..5 {
        let mut data = [0u8; 250];
        bincode::encode_into_slice(&message, &mut data, bincode::config::standard()).unwrap();
        let status = esp_now.send(&BROADCAST_ADDRESS, &data).unwrap().wait();
        println!("Send broadcast status: {:?}", status)
    }
}
