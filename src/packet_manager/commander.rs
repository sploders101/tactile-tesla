extern crate alloc;

use crate::{
    binary_packets::{PacketReader, PacketWriter},
    hw_aes::{self, AES_KEY_SIZE},
    hw_hmac::{self, authenticate_packet},
    packetizer::{PacketAssembler, TolerantPacketDisassembler, TOLERANT_PACKET_OVERHEAD},
};
use alloc::vec::Vec;
use esp_hal::{
    aes::Aes,
    rng::Rng,
    sha::Sha,
    time::{self, Duration, Instant},
};
use esp_println::println;
use esp_wifi::esp_now::{EspNow, BROADCAST_ADDRESS, ESP_NOW_MAX_DATA_LEN};

use super::CLUSTER_SECRET;

const INNER_PACKET_MAX_LEN: usize = ESP_NOW_MAX_DATA_LEN - hw_hmac::HASH_SIZE;
const MAX_NODES: usize = 50;

struct ClientPacketizer {
    last_heartbeat: Instant,
    packetizer: PacketAssembler,
}

pub struct CommanderPacketManager<'a> {
    esp_now: EspNow<'a>,
    next_heartbeat: Instant,
    packet_disassembler: TolerantPacketDisassembler<INNER_PACKET_MAX_LEN>,
    packetizers: heapless::Vec<([u8; 6], ClientPacketizer), MAX_NODES>,
}
impl<'a> CommanderPacketManager<'a> {
    pub fn new(esp_now: EspNow<'a>) -> Self {
        return CommanderPacketManager {
            esp_now,
            next_heartbeat: time::now(),
            packet_disassembler: TolerantPacketDisassembler::new(),
            packetizers: heapless::Vec::new(),
        };
    }

    /// Sends a broadcast via esp-now.
    ///
    /// NOTE: Packet may have garbage data appended to the end due to
    /// encryption semantics. If this is not okay, a length indicator
    /// should be added to the packet
    pub fn broadcast_packet(
        &mut self,
        aes_peripheral: &mut Aes<'_>,
        sha_peripheral: &mut Sha<'_>,
        rng_peripheral: &mut Rng,
        mut packet: Vec<u8>,
    ) {
        // Encrypt packet in-place
        hw_aes::encrypt_packet(
            aes_peripheral,
            rng_peripheral,
            &CLUSTER_SECRET[0..AES_KEY_SIZE].try_into().unwrap(),
            &mut packet,
        );

        // Split packet into chunks for transport
        let mut chunk_iter = self.packet_disassembler.split_packet(&packet);
        let mut chunk = [0u8; ESP_NOW_MAX_DATA_LEN];
        while let Some(bytes_written) =
            chunk_iter.get_chunk((&mut chunk[hw_hmac::HASH_SIZE..]).try_into().unwrap())
        {
            // println!("Pre-hash chunk: {:02X?}", &chunk);
            // Add HMAC to each packet to prove authenticity
            let hmac = hw_hmac::hmac_cluster_chunk(
                sha_peripheral,
                &chunk[hw_hmac::HASH_SIZE..hw_hmac::HASH_SIZE + bytes_written],
            );
            // This can be accelerated if hmac_cluster_chunk could write directly to chunk
            for i in 0..hw_hmac::HASH_SIZE {
                chunk[i] = hmac[i];
            }

            // Send packet on broadcast channel
            let to_send = &chunk[0..hw_hmac::HASH_SIZE + bytes_written];
            println!("Sending packet: {:02X?}", to_send);
            if let Some(_) = authenticate_packet(sha_peripheral, to_send) {
                println!("Authentication passed!");
            } else {
                println!("Authentication failed.");
            }
            self.esp_now
                .send(&BROADCAST_ADDRESS, to_send)
                .unwrap()
                .wait()
                .ok();
        }
    }

    pub fn tick(
        &mut self,
        aes_peripheral: &mut Aes<'_>,
        sha_peripheral: &mut Sha<'_>,
        rng_peripheral: &mut Rng,
    ) {
        let tick_now = time::now();

        // Send heartbeat if necessary
        if tick_now >= self.next_heartbeat {
            self.next_heartbeat = tick_now + Duration::secs(2);
            let mut packet = PacketWriter::new();
            packet.write_cluster_id();
            packet.write_u8(0);
            self.broadcast_packet(
                aes_peripheral,
                sha_peripheral,
                rng_peripheral,
                packet.finish(),
            );
        }

        // // Receive buffered packets
        // while let Some(data) = self.esp_now.receive() {
        //     let chunk = &data.data[0..data.len as usize];

        //     // Handle broadcast packets
        //     if data.info.dst_address == BROADCAST_ADDRESS {
        //         let mut packet_reader = PacketReader::new(chunk);
        //         if !packet_reader.verify_cluster_id() {
        //             continue;
        //         }
        //     }
        // }
    }
}
