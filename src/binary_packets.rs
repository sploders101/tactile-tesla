extern crate alloc;
extern crate core;
use alloc::vec::Vec;
use core::str::{self, Utf8Error};

#[derive(Debug, Clone)]
pub enum PacketWriteError {
    /// The data is too large to include in the packet
    TooLarge,
}

/// Builds packets via manual assembly
pub struct PacketWriter {
    packet: Vec<u8>,
}
impl PacketWriter {
    pub fn new() -> Self {
        return Self { packet: Vec::new() };
    }

    pub fn write_u8(&mut self, num: u8) {
        self.packet.push(num);
    }
    pub fn write_u16(&mut self, num: u16) {
        self.packet.extend_from_slice(&num.to_be_bytes());
    }
    pub fn write_u32(&mut self, num: u32) {
        self.packet.extend_from_slice(&num.to_be_bytes());
    }
    pub fn write_u64(&mut self, num: u64) {
        self.packet.extend_from_slice(&num.to_be_bytes());
    }
    pub fn write_u128(&mut self, num: u128) {
        self.packet.extend_from_slice(&num.to_be_bytes());
    }

    pub fn write_i8(&mut self, num: i8) {
        self.packet.push(unsafe { core::mem::transmute(num) });
    }
    pub fn write_i16(&mut self, num: i16) {
        self.packet.extend_from_slice(&num.to_be_bytes());
    }
    pub fn write_i32(&mut self, num: i32) {
        self.packet.extend_from_slice(&num.to_be_bytes());
    }
    pub fn write_i64(&mut self, num: i64) {
        self.packet.extend_from_slice(&num.to_be_bytes());
    }
    pub fn write_i128(&mut self, num: i128) {
        self.packet.extend_from_slice(&num.to_be_bytes());
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), PacketWriteError> {
        let length = bytes.len();
        if length > u16::MAX as _ {
            return Err(PacketWriteError::TooLarge);
        }
        self.write_u16(length as _);
        self.packet.extend_from_slice(bytes);
        return Ok(());
    }
    pub fn write_str(&mut self, string: &str) -> Result<(), PacketWriteError> {
        return self.write_bytes(string.as_bytes());
    }

    pub fn finish(self) -> Vec<u8> {
        return self.packet;
    }
}

/// Builds packets via manual assembly
pub struct PacketReader<'a> {
    cursor: usize,
    packet: &'a [u8],
}
impl<'a> PacketReader<'a> {
    pub fn new(packet: &'a [u8]) -> Self {
        return Self { cursor: 0, packet };
    }

    pub fn read_u8(&mut self) -> Option<u8> {
        if self.cursor + 1 > self.packet.len() {
            return None;
        }
        let num = self.packet[self.cursor];
        self.cursor += 1;
        return Some(num);
    }
    pub fn read_u16(&mut self) -> Option<u16> {
        if self.cursor + 2 > self.packet.len() {
            return None;
        }
        let num = u16::from_be_bytes([self.packet[self.cursor], self.packet[self.cursor + 1]]);
        self.cursor += 2;
        return Some(num);
    }
    pub fn read_u32(&mut self) -> Option<u32> {
        if self.cursor + 4 > self.packet.len() {
            return None;
        }
        let num = u32::from_be_bytes([
            self.packet[self.cursor],
            self.packet[self.cursor + 1],
            self.packet[self.cursor + 2],
            self.packet[self.cursor + 3],
        ]);
        self.cursor += 4;
        return Some(num);
    }
    pub fn read_u64(&mut self) -> Option<u64> {
        if self.cursor + 8 > self.packet.len() {
            return None;
        }
        let num = u64::from_be_bytes([
            self.packet[self.cursor],
            self.packet[self.cursor + 1],
            self.packet[self.cursor + 2],
            self.packet[self.cursor + 3],
            self.packet[self.cursor + 4],
            self.packet[self.cursor + 5],
            self.packet[self.cursor + 6],
            self.packet[self.cursor + 7],
        ]);
        self.cursor += 8;
        return Some(num);
    }
    pub fn read_u128(&mut self) -> Option<u128> {
        if self.cursor + 16 > self.packet.len() {
            return None;
        }
        let num = u128::from_be_bytes([
            self.packet[self.cursor],
            self.packet[self.cursor + 1],
            self.packet[self.cursor + 2],
            self.packet[self.cursor + 3],
            self.packet[self.cursor + 4],
            self.packet[self.cursor + 5],
            self.packet[self.cursor + 6],
            self.packet[self.cursor + 7],
            self.packet[self.cursor + 8],
            self.packet[self.cursor + 9],
            self.packet[self.cursor + 10],
            self.packet[self.cursor + 11],
            self.packet[self.cursor + 12],
            self.packet[self.cursor + 13],
            self.packet[self.cursor + 14],
            self.packet[self.cursor + 15],
        ]);
        self.cursor += 16;
        return Some(num);
    }

    pub fn read_i8(&mut self) -> Option<i8> {
        if self.cursor + 1 > self.packet.len() {
            return None;
        }
        let num = unsafe { core::mem::transmute(self.packet[self.cursor]) };
        self.cursor += 1;
        return Some(num);
    }
    pub fn read_i16(&mut self) -> Option<i16> {
        if self.cursor + 2 > self.packet.len() {
            return None;
        }
        let num = i16::from_be_bytes([self.packet[self.cursor], self.packet[self.cursor + 1]]);
        self.cursor += 2;
        return Some(num);
    }
    pub fn read_i32(&mut self) -> Option<i32> {
        if self.cursor + 4 > self.packet.len() {
            return None;
        }
        let num = i32::from_be_bytes([
            self.packet[self.cursor],
            self.packet[self.cursor + 1],
            self.packet[self.cursor + 2],
            self.packet[self.cursor + 3],
        ]);
        self.cursor += 4;
        return Some(num);
    }
    pub fn read_i64(&mut self) -> Option<i64> {
        if self.cursor + 8 > self.packet.len() {
            return None;
        }
        let num = i64::from_be_bytes([
            self.packet[self.cursor],
            self.packet[self.cursor + 1],
            self.packet[self.cursor + 2],
            self.packet[self.cursor + 3],
            self.packet[self.cursor + 4],
            self.packet[self.cursor + 5],
            self.packet[self.cursor + 6],
            self.packet[self.cursor + 7],
        ]);
        self.cursor += 8;
        return Some(num);
    }
    pub fn read_i128(&mut self) -> Option<i128> {
        if self.cursor + 16 > self.packet.len() {
            return None;
        }
        let num = i128::from_be_bytes([
            self.packet[self.cursor],
            self.packet[self.cursor + 1],
            self.packet[self.cursor + 2],
            self.packet[self.cursor + 3],
            self.packet[self.cursor + 4],
            self.packet[self.cursor + 5],
            self.packet[self.cursor + 6],
            self.packet[self.cursor + 7],
            self.packet[self.cursor + 8],
            self.packet[self.cursor + 9],
            self.packet[self.cursor + 10],
            self.packet[self.cursor + 11],
            self.packet[self.cursor + 12],
            self.packet[self.cursor + 13],
            self.packet[self.cursor + 14],
            self.packet[self.cursor + 15],
        ]);
        self.cursor += 16;
        return Some(num);
    }

    pub fn read_bytes(&mut self) -> Option<&[u8]> {
        let length = self.read_u16()? as usize;
        if self.cursor + length > self.packet.len() {
            return None;
        }
        let data = &self.packet[self.cursor..self.cursor + length];
        self.cursor += length;
        return Some(data);
    }
    pub fn read_str(&mut self) -> Option<Result<&str, Utf8Error>> {
        return Some(str::from_utf8(self.read_bytes()?));
    }

    pub fn get_remainder(self) -> &'a [u8] {
        return &self.packet[self.cursor..];
    }
}
