//! ESP-NOW Example
//!
//! Broadcasts, receives and sends messages via esp-now

//% FEATURES: esp-wifi esp-wifi/wifi-default esp-wifi/wifi esp-wifi/utils esp-wifi/esp-now
//% CHIPS: esp32 esp32s2 esp32s3 esp32c2 esp32c3 esp32c6

#![no_std]
#![no_main]

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{aes::Aes, prelude::*, rng::Rng, sha::Sha, timer::timg::TimerGroup};
use esp_println::println;
use esp_wifi::{init, EspWifiInitFor};
use tactile_tesla::packet_manager::{PacketManager, Role};

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

    let mut rng = Rng::new(peripherals.RNG);
    let init = init(
        EspWifiInitFor::Wifi,
        timg0.timer0,
        rng.clone(),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let esp_now = esp_wifi::esp_now::EspNow::new(&init, wifi).unwrap();
    let mut aes = Aes::new(peripherals.AES);
    let mut sha = Sha::new(peripherals.SHA);

    println!("esp-now version {}", esp_now.get_version().unwrap());

    let mut manager = PacketManager::new(esp_now);
    loop {
        manager.tick(&mut aes, &mut sha, &mut rng, Role::Commander);
    }
}
