extern crate alloc;

use alloc::vec::Vec;
use esp_hal::{aes::Aes, rng::Rng};
use thiserror::Error;

pub const AES_BLOCK_SIZE: usize = 16;
pub const IV_SIZE: usize = AES_BLOCK_SIZE;
pub const AES_KEY_SIZE: usize = 32;

/// Encrypts a packet using AES-256.
///
/// IV is randomly generated and appended to the end of the packet.
/// Garbled data may appear at the end of the packet after decryption.
/// Trimming should be done through other means.
pub fn encrypt_packet(
    aes_peripheral: &mut Aes<'_>,
    rng_peripheral: &mut Rng,
    key: &[u8; AES_KEY_SIZE],
    packet: &mut Vec<u8>,
) {
    // Generate IV
    let mut iv = [0u8; IV_SIZE];
    rng_peripheral.read(&mut iv);

    let mut cursor = 0;

    // Ensure proper block size and allocate room to add the IV later
    let padding = AES_BLOCK_SIZE - (packet.len() % AES_BLOCK_SIZE);
    packet.reserve_exact(padding + IV_SIZE);
    for _ in 0..padding {
        packet.push(0);
    }

    while cursor < packet.len() {
        // Scramble
        if cursor == 0 {
            for i in 0..AES_BLOCK_SIZE {
                // XOR against IV for first packet
                packet[i] = packet[i] ^ iv[i];
            }
        } else {
            for i in 0..AES_BLOCK_SIZE {
                // XOR against previous encrypted packet
                packet[i+cursor] = packet[i+cursor] ^ packet[i+cursor-AES_BLOCK_SIZE];
            }
        }

        // Encrypt
        let packet_block: &mut [u8; AES_BLOCK_SIZE] = (&mut packet
            [cursor..cursor + AES_BLOCK_SIZE])
            .try_into()
            .unwrap();
        aes_peripheral.process(packet_block, esp_hal::aes::Mode::Encryption256, key.clone());

        // Increment cursor
        cursor += AES_BLOCK_SIZE;
    }

    // Append IV to the end of the packet
    packet.extend_from_slice(&iv);
}

#[derive(Error, Debug, Clone)]
pub enum DecryptionError {
    #[error("The packet to be decrypted was not the correct length")]
    InvalidLength,
}

pub fn decrypt_packet<'a>(
    aes_peripheral: &mut Aes<'_>,
    key: &[u8; AES_KEY_SIZE],
    packet: &'a mut [u8],
) -> Result<&'a mut [u8], DecryptionError> {
    if packet.len() % AES_BLOCK_SIZE != 0 {
        return Err(DecryptionError::InvalidLength);
    }

    // Get IV offset
    let iv_start = packet.len() - IV_SIZE;

    // Set up vector for CBC
    let mut prev_encrypted_block = [0u8; IV_SIZE];
    for i in 0..IV_SIZE {
        prev_encrypted_block[i] = packet[iv_start + i];
    }

    let packet: &mut [u8] = &mut packet[0..iv_start];

    let mut cursor = 0;

    while cursor < packet.len() {
        let packet_block: &mut [u8; AES_BLOCK_SIZE] = (&mut packet
            [cursor..cursor + AES_BLOCK_SIZE])
            .try_into()
            .unwrap();
        let encrypted_packet_block = packet_block.clone();

        // Decrypt
        aes_peripheral.process(packet_block, esp_hal::aes::Mode::Decryption256, key.clone());

        // Unscramble
        for i in 0..AES_BLOCK_SIZE {
            packet_block[i] = packet_block[i] ^ prev_encrypted_block[i];
        }

        // Save encrypted block for next iteration
        prev_encrypted_block = encrypted_packet_block;

        // Increment cursor
        cursor += AES_BLOCK_SIZE;
    }

    return Ok(packet);
}
