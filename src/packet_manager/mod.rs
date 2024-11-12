extern crate alloc;

use crate::{
    binary_packets::{PacketReader, PacketWriter},
    hw_aes::{self, AES_KEY_SIZE},
    hw_hmac::{self},
    packet_types::{CommPacket, Heartbeat, Transmittable},
    packetizer::{TolerantPacketAssembler, TolerantPacketDisassembler},
};
use alloc::{string::String, vec::Vec};
use esp_hal::{
    aes::Aes,
    rng::Rng,
    sha::Sha,
    time::{self, Duration, Instant},
};
use esp_println::println;
use esp_wifi::esp_now::{EspNow, BROADCAST_ADDRESS, ESP_NOW_MAX_DATA_LEN};

static CLUSTER_KEY: &'static [u8] = include_bytes!("../../keys/cluster_key.dat");
const INNER_PACKET_MAX_LEN: usize = ESP_NOW_MAX_DATA_LEN - hw_hmac::HASH_SIZE;
const MAX_NODES: usize = 50;

pub enum Role {
    /// This is the main controller with access to the vehicle CANbus
    Commander,
    /// This is a generic node with no special requirements
    Node,
}

struct PeerPacketizer {
    last_heartbeat: Instant,
    packetizer: TolerantPacketAssembler,
}
impl PeerPacketizer {
    pub fn new() -> Self {
        return Self {
            last_heartbeat: time::now(),
            packetizer: TolerantPacketAssembler::new(),
        };
    }
}

pub struct PacketManager<'a> {
    esp_now: EspNow<'a>,
    next_heartbeat: Instant,
    packet_disassembler: TolerantPacketDisassembler<INNER_PACKET_MAX_LEN>,
    packetizers: heapless::Vec<([u8; 6], PeerPacketizer), MAX_NODES>,
}
impl<'a> PacketManager<'a> {
    pub fn new(esp_now: EspNow<'a>) -> Self {
        return PacketManager {
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
    ///
    /// Packets are:
    /// 1. Encrypted
    /// 2. Chunked
    /// 3. HMAC'd
    /// 4. Sent
    fn broadcast_packet(
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
            &CLUSTER_KEY[0..AES_KEY_SIZE].try_into().unwrap(),
            &mut packet,
        );

        // Split packet into chunks for transport
        let mut chunk_iter = self.packet_disassembler.split_packet(&packet);
        let mut chunk = [0u8; ESP_NOW_MAX_DATA_LEN];
        while let Some(bytes_written) =
            chunk_iter.get_chunk((&mut chunk[hw_hmac::HASH_SIZE..]).try_into().unwrap())
        {
            // Add HMAC to each packet to prove authenticity
            hw_hmac::hmac_cluster_chunk(
                sha_peripheral,
                &chunk[hw_hmac::HASH_SIZE..hw_hmac::HASH_SIZE + bytes_written],
            )
            .into_iter()
            .enumerate()
            .for_each(|(i, byte)| chunk[i] = byte);

            // Send packet on broadcast channel
            let to_send = &chunk[0..hw_hmac::HASH_SIZE + bytes_written];
            self.esp_now
                .send(&BROADCAST_ADDRESS, to_send)
                .unwrap()
                .wait()
                .ok();
        }
    }

    /// Adds a chunk to the sender's context for processing and returns a packet if one
    /// was completed. Performs HMAC verification, assembly, and decryption.
    fn unwrap_packet<T: Transmittable>(
        &mut self,
        aes_peripheral: &mut Aes<'_>,
        sha_peripheral: &mut Sha<'_>,
        sender_mac: &[u8; 6],
        packet: &[u8],
    ) -> Option<T> {
        // Check that the packet can accomodate an HMAC.
        // If not, it's not one of ours.
        if packet.len() < hw_hmac::HASH_SIZE + 1 {
            return None;
        }

        // Verify HMAC
        let computed_hmac =
            hw_hmac::hmac_cluster_chunk(sha_peripheral, &packet[hw_hmac::HASH_SIZE..]);
        for i in 0..hw_hmac::HASH_SIZE {
            if packet[i] != computed_hmac[i] {
                // HMAC doesn't match, so it's not one of ours.
                return None;
            }
        }

        // Get sender's context
        let sender_ctx = match self.packetizers.iter_mut().find(|i| i.0 == *sender_mac) {
            Some(sender_ctx) => &mut sender_ctx.1,
            None => {
                if let Err(_) = self
                    .packetizers
                    .push((sender_mac.clone(), PeerPacketizer::new()))
                {
                    log::error!("More than {MAX_NODES} found. Dropping packets.");
                    return None;
                }
                &mut self.packetizers.last_mut().unwrap().1
            }
        };

        sender_ctx.last_heartbeat = time::now();
        sender_ctx
            .packetizer
            .push_data(&packet[hw_hmac::HASH_SIZE..]);

        return match sender_ctx.packetizer.next() {
            Some(mut packet) => {
                // Decrypt packet in-place
                if let Ok(packet) = hw_aes::decrypt_packet(
                    aes_peripheral,
                    &CLUSTER_KEY[0..AES_KEY_SIZE].try_into().unwrap(),
                    &mut packet,
                ) {
                    T::decode(&mut PacketReader::new(packet))
                } else {
                    None
                }
            }
            None => None,
        };
    }

    pub fn tick(
        &mut self,
        aes_peripheral: &mut Aes<'_>,
        sha_peripheral: &mut Sha<'_>,
        rng_peripheral: &mut Rng,
        role_hint: Role,
    ) {
        let tick_now = time::now();

        // Send heartbeat if necessary
        if tick_now >= self.next_heartbeat {
            self.next_heartbeat = tick_now + Duration::secs(2);
            let packet = CommPacket::Heartbeat(Heartbeat {
                car_name: Some(unsafe {
                    String::from_utf8_unchecked(Vec::from(include_bytes!(
                        "../../keys/car_name.txt"
                    )))
                }),
            });
            let mut packet_bytes = PacketWriter::new();
            packet.encode(&mut packet_bytes).unwrap();
            self.broadcast_packet(
                aes_peripheral,
                sha_peripheral,
                rng_peripheral,
                packet_bytes.finish(),
            );
        }

        // // Receive buffered packets
        while let Some(data) = self.esp_now.receive() {
            let chunk = &data.data[0..data.len as usize];

            let started = time::now();
            if let Some(packet) = self.unwrap_packet::<CommPacket>(
                aes_peripheral,
                sha_peripheral,
                &data.info.src_address,
                chunk,
            ) {
                let elapsed = time::now() - started;
                println!("Decryption took {}", elapsed);
                println!("Got packet: {:?}", packet);
            }
        }
    }
}
