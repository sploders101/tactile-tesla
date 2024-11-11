//! ESP-NOW Example
//!
//! Broadcasts, receives and sends messages via esp-now

//% FEATURES: esp-wifi esp-wifi/wifi-default esp-wifi/wifi esp-wifi/utils esp-wifi/esp-now
//% CHIPS: esp32 esp32s2 esp32s3 esp32c2 esp32c3 esp32c6

#![no_std]

pub mod binary_packets;
pub mod hw_aes;
pub mod hw_hmac;
pub mod packet_manager;
pub mod packet_types;
pub mod packetizer;

use bincode::{Decode, Encode};

#[derive(Encode, Decode, Debug, Clone)]
pub enum NowMessage {
    Speedometer { speed: u16 },
}
