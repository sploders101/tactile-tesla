extern crate alloc;

use alloc::string::String;
use crate::{
    binary_packets::{PacketReader, PacketWriteError, PacketWriter},
    packet_manager::Role,
};

pub trait Transmittable: Sized {
    fn encode(&self, packet_writer: &mut PacketWriter) -> Result<(), PacketWriteError>;
    fn decode(packet_reader: &mut PacketReader) -> Option<Self>;
}

impl Transmittable for Role {
    fn encode(&self, packet_writer: &mut PacketWriter) -> Result<(), PacketWriteError> {
        match self {
            Self::Commander => {
                packet_writer.write_u8(0);
            }
            Self::Node => {
                packet_writer.write_u8(1);
            }
        }
        return Ok(());
    }
    fn decode(packet_reader: &mut PacketReader) -> Option<Self> {
        match packet_reader.read_u8()? {
            0 => Some(Self::Commander),
            1 => Some(Self::Node),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CommPacket {
    Heartbeat(Heartbeat),
}
impl Transmittable for CommPacket {
    fn encode(&self, packet_writer: &mut PacketWriter) -> Result<(), PacketWriteError> {
        match self {
            Self::Heartbeat(heartbeat) => {
                packet_writer.write_u8(0);
                return heartbeat.encode(packet_writer);
            }
        }
    }
    fn decode(packet_reader: &mut PacketReader) -> Option<Self> {
        match packet_reader.read_u8()? {
            0 => Some(Self::Heartbeat(Heartbeat::decode(packet_reader)?)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Heartbeat {
    pub car_name: Option<String>,
}
impl Transmittable for Heartbeat {
    fn encode(&self, packet_writer: &mut PacketWriter) -> Result<(), PacketWriteError> {
        match self.car_name {
            None => {
                packet_writer.write_u8(0);
            }
            Some(ref car_name) => {
                packet_writer.write_u8(1);
                packet_writer.write_str(car_name.as_str())?;
            }
        }
        return Ok(());
    }
    fn decode(packet_reader: &mut PacketReader) -> Option<Self> {
        let car_name = match packet_reader.read_u8()? {
            0 => None,
            1 => Some(String::from(packet_reader.read_str()?.ok()?)),
            _ => return None,
        };
        return Some(Self {
            car_name,
        });
    }
}
