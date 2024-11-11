extern crate alloc;
use esp_hal::{
    aes,
    time::{self, Duration, Instant},
};
use esp_wifi::esp_now::{EspNow, BROADCAST_ADDRESS};
use heapless::Vec;

use crate::{
    binary_packets::{PacketReader, PacketWriter},
    packetizer::AuthenticatedPacketizer,
};

const MAX_NODES: usize = 50;

struct ClientPacketizer {
    last_heartbeat: Instant,
    packetizer: AuthenticatedPacketizer,
}

pub struct CommanderPacketManager<'a> {
    esp_now: EspNow<'a>,
    next_heartbeat: Instant,
    packetizers: Vec<([u8; 6], ClientPacketizer), MAX_NODES>,
}
impl<'a> CommanderPacketManager<'a> {
    pub fn new(esp_now: EspNow<'a>) -> Self {
        return CommanderPacketManager {
            esp_now,
            next_heartbeat: time::now(),
            packetizers: Vec::new(),
        };
    }

    pub fn tick(&mut self) {
        let tick_now = time::now();

        // Send heartbeat if necessary
        if tick_now >= self.next_heartbeat {
            self.next_heartbeat = tick_now + Duration::secs(2);
            let mut packet = PacketWriter::new();
            packet.write_cluster_id();
            packet.write_u8(0);
            let _ = self.esp_now.send(&BROADCAST_ADDRESS, &packet.finish());
        }

        // Receive buffered packets
        while let Some(data) = self.esp_now.receive() {
            let chunk = &data.data[0..data.len as usize];

            // Handle broadcast packets
            if data.info.dst_address == BROADCAST_ADDRESS {
                let mut packet_reader = PacketReader::new(chunk);
                if !packet_reader.verify_cluster_id() {
                    continue;
                }
            }
        }
    }
}
