extern crate alloc;
extern crate core;

use crate::binary_packets::PacketReader;
use alloc::{collections::VecDeque, vec::Vec};
use core::cmp::min;

// 4 bytes for msg_seq, 2 for chunk_seq
pub const TOLERANT_PACKET_OVERHEAD: usize = 6;

/// Disassembles a large packet into smaller ones.
///
/// Works with unreliable transports by adding sequencing
/// information to each packet. This signals to the client
/// that the packet should be dropped since it was not received
/// in full.
///
/// This is the transmitting end of `TolerantPacketReassembler`.
pub struct TolerantPacketDisassembler<const MAX_CHUNK_SIZE: usize> {
    msg_seq: u32,
}
impl<const MAX_CHUNK_SIZE: usize> TolerantPacketDisassembler<MAX_CHUNK_SIZE> {
    pub fn new() -> Self {
        if MAX_CHUNK_SIZE < TOLERANT_PACKET_OVERHEAD + 1 {
            panic!("Cannot instantiate disassembler. Chunk size too small.");
        }
        return Self { msg_seq: 0 };
    }

    pub fn split_packet<'a>(
        &mut self,
        packet: &'a [u8],
    ) -> TolerantPacketIterator<'a, MAX_CHUNK_SIZE> {
        self.msg_seq += 1;
        return TolerantPacketIterator::<'a, MAX_CHUNK_SIZE> {
            msg_seq: self.msg_seq,
            chunk_seq: 0,
            cursor: 0,
            data: packet,
        };
    }
}

/// Yields chunks of a packet.
///
/// Meant to be created via `TolerantPacketDisassembler::split_packet`
pub struct TolerantPacketIterator<'a, const MAX_CHUNK_SIZE: usize> {
    msg_seq: u32,
    chunk_seq: u16,
    cursor: usize,
    data: &'a [u8],
}
impl<'a, const MAX_CHUNK_SIZE: usize> TolerantPacketIterator<'a, MAX_CHUNK_SIZE> {
    const ADVANCE_COUNT: usize = MAX_CHUNK_SIZE - TOLERANT_PACKET_OVERHEAD;

    /// Writes the next data chunk to `chunk`.
    ///
    /// Returns the number of bytes written, if any.
    pub fn get_chunk(&mut self, chunk: &mut [u8; MAX_CHUNK_SIZE]) -> Option<usize> {
        if self.cursor >= self.data.len() {
            return None;
        }
        self.chunk_seq += 1;
        let msg_seq_bytes = self.msg_seq.to_be_bytes();
        let chunk_seq_bytes = self.chunk_seq.to_be_bytes();
        for i in 0..msg_seq_bytes.len() {
            chunk[i] = msg_seq_bytes[i];
        }
        for i in 0..chunk_seq_bytes.len() {
            chunk[i + msg_seq_bytes.len()] = chunk_seq_bytes[i];
        }
        let start_cursor = self.cursor;
        let end_cursor = min(self.cursor + Self::ADVANCE_COUNT, self.data.len());
        self.cursor = end_cursor;
        for (i, chunk_i) in (start_cursor..end_cursor).enumerate() {
            chunk[chunk_i + TOLERANT_PACKET_OVERHEAD] = self.data[i];
        }
        return Some(end_cursor - start_cursor + TOLERANT_PACKET_OVERHEAD);
    }
}

/// Re-assembles chunked packets into their original form.
///
/// Works with unreliable transports, dropping the in-progress
/// packet when an error is detected. This is lossy, and does
/// not request retransmission.
///
/// This is the receiving end of `TolerantPacketDisassembler`.
pub struct TolerantPacketReassembler {
    msg_seq: u32,
    chunk_seq: u16,
    assembler: PacketAssembler,
}
impl TolerantPacketReassembler {
    pub fn new() -> Self {
        return Self {
            msg_seq: 0,
            chunk_seq: 0,
            assembler: PacketAssembler::new(),
        };
    }

    pub fn push_data(&mut self, chunk: &[u8]) {
        let mut packet_reader = PacketReader::new(chunk);
        let msg_seq = match packet_reader.read_u32() {
            Some(msg_seq) => msg_seq,
            None => return,
        };
        let chunk_seq = match packet_reader.read_u16() {
            Some(msg_seq) => msg_seq,
            None => return,
        };
        let data = packet_reader.get_remainder();

        // Skip repeated messages
        if self.msg_seq >= msg_seq || self.chunk_seq >= chunk_seq {
            return;
        }

        // If we found a new message, clear the buffers of incomplete ones
        if self.msg_seq < msg_seq {
            self.msg_seq = msg_seq;
            self.chunk_seq = 0;
            self.assembler.expected_size = ExpectedSize::None;
            self.assembler.buffer = Vec::new();
        }

        // If we missed a packet, don't keep trying to parse the message
        if self.chunk_seq + 1 < chunk_seq {
            return;
        }

        // If all checks cleared, go ahead and push the message onto the queue
        self.assembler.push_data(data);
    }
}

impl Iterator for TolerantPacketReassembler {
    type Item = Vec<u8>;
    fn next(&mut self) -> Option<Self::Item> {
        return self.assembler.next();
    }
}

enum ExpectedSize {
    None,
    Partial(u8),
    Full(u16),
}

/// Re-assembles chunked packets into their original form.
///
/// Assumes reliable ordered transport.
///
/// Does not make any assumptions whatsoever about chunk boundaries.
pub struct PacketAssembler {
    packets: VecDeque<Vec<u8>>,
    expected_size: ExpectedSize,
    buffer: Vec<u8>,
}
impl PacketAssembler {
    pub fn new() -> Self {
        return Self {
            packets: VecDeque::new(),
            expected_size: ExpectedSize::None,
            buffer: Vec::new(),
        };
    }

    pub fn push_data(&mut self, mut chunk: &[u8]) {
        // Using a loop here because our stack isn't very big and recursion is a memory hog
        loop {
            if chunk.len() == 0 {
                return;
            }
            match self.expected_size {
                ExpectedSize::None => {
                    if chunk.len() >= 2 {
                        self.expected_size =
                            ExpectedSize::Full(u16::from_be_bytes([chunk[0], chunk[1]]));
                        chunk = &chunk[2..];
                        continue;
                    } else {
                        self.expected_size = ExpectedSize::Partial(chunk[0]);
                    }
                }
                ExpectedSize::Partial(byte1) => {
                    self.expected_size = ExpectedSize::Full(u16::from_be_bytes([byte1, chunk[0]]));
                    chunk = &chunk[1..];
                    continue;
                }
                ExpectedSize::Full(expected_size) => {
                    if self.buffer.len() + chunk.len() > expected_size as usize {
                        self.buffer.extend_from_slice(
                            &chunk[0..expected_size as usize - self.buffer.len()],
                        );
                        self.packets.push_back(core::mem::take(&mut self.buffer));
                        chunk = &chunk[expected_size as usize - self.buffer.len()..];
                        continue;
                    } else {
                        self.buffer.extend_from_slice(chunk);
                        if self.buffer.len() == expected_size as usize {
                            self.packets.push_back(core::mem::take(&mut self.buffer));
                        }
                    }
                }
            }
        }
    }
}

impl Iterator for PacketAssembler {
    type Item = Vec<u8>;
    fn next(&mut self) -> Option<Self::Item> {
        return self.packets.pop_front();
    }
}
