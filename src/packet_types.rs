extern crate alloc;
extern crate core;

use crate::binary_packets::{PacketReader, PacketWriter};

pub static CLUSTER_ID: &'static [u8; 6] = include_bytes!("../keys/cluster_id.dat");

pub trait Transmittable: Sized + Sync + Send + 'static {
    fn encode(&self, builder: &mut PacketWriter);
    fn decode(extractor: &mut PacketReader) -> Option<Self>;
}

pub enum BcastMessageType {
    Discovery,
}
impl Transmittable for BcastMessageType {
    fn encode(&self, builder: &mut PacketWriter) {
        match self {
            Self::Discovery => {
                builder.write_u8(1);
                // Add cluster ID to packet to help reduce noise
                // on the receiving end.
                for byte in CLUSTER_ID {
                    builder.write_u8(*byte);
                }
            }
        }
    }
    fn decode(reader: &mut PacketReader) -> Option<Self> {
        match reader.read_u8()? {
            1 => {
                // Check that this device is part of our cluster
                // before accepting the discovery packet.
                for i in 0..CLUSTER_ID.len() {
                    let byte = reader.read_u8()?;
                    if byte != CLUSTER_ID[i] {
                        return None;
                    }
                }
                Some(Self::Discovery)
            }
            _ => None,
        }
    }
}

pub enum UnicastMessageType {
    /// Contains data from the peripheral to be signed and returned
    AuthChallenge([u8; 256]),
    /// Contains plain-text data as a response to a challenge
    AuthResponse([u8; 200]),
}
