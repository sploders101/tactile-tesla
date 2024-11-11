extern crate alloc;

use alloc::vec::Vec;
use esp_hal::{aes::Aes, rng::Rng};
use thiserror::Error;

const IV_SIZE: usize = 16;
const AES_BLOCK_SIZE: usize = 16;
const AES_KEY_SIZE: usize = 32;

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
    // Generate IV and calculate initial key
    let mut iv = [0u8; IV_SIZE];
    rng_peripheral.read(&mut iv);
    let mut key_buf = [0u8; AES_KEY_SIZE];
    // Avoid expensive modulo in a hot path
    let mut iv_cursor = 0;
    for i in 0..AES_KEY_SIZE {
        // XOR key with tiled iv_cursor to get packet key
        key_buf[i] = key[i] ^ iv[iv_cursor];
        iv_cursor += 1;
        if iv_cursor > IV_SIZE {
            iv_cursor = 0;
        }
    }

    let mut cursor = 0;

    // Ensure proper block size and allocate room to add the IV later
    let padding = packet.len() % AES_BLOCK_SIZE;
    packet.reserve_exact(padding + IV_SIZE);
    for _ in 0..padding {
        packet.push(0);
    }

    while cursor < packet.len() {
        let packet_block: &mut [u8; AES_BLOCK_SIZE] = (&mut packet
            [cursor..cursor + AES_BLOCK_SIZE])
            .try_into()
            .unwrap();

        // Calculate next key from unencrypted data
        let mut next_key = [0u8; AES_KEY_SIZE];
        let mut block_cursor = 0;
        for i in 0..AES_KEY_SIZE {
            next_key[i] = key_buf[i] ^ packet_block[block_cursor];
            block_cursor += 1;
            if block_cursor > AES_BLOCK_SIZE {
                block_cursor = 0;
            }
        }

        // Encrypt
        aes_peripheral.process(packet_block, esp_hal::aes::Mode::Encryption256, key_buf);

        // Set new key
        key_buf = next_key;

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
    if (packet.len() - IV_SIZE) % AES_BLOCK_SIZE != 0 {
        return Err(DecryptionError::InvalidLength);
    }

    // Calculate starting key from key and packet iv
    let mut key_buf = [0u8; AES_KEY_SIZE];
    let iv_start = packet.len() - IV_SIZE;
    let iv_buf: &[u8; IV_SIZE] = packet[iv_start..].try_into().unwrap();
    let mut iv_cursor = 0;
    for i in 0..AES_KEY_SIZE {
        key_buf[i] = key[i] ^ iv_buf[iv_cursor];
        iv_cursor += 1;
        if iv_cursor > IV_SIZE {
            iv_cursor = 0;
        }
    }

    let packet: &mut [u8] = &mut packet[0..iv_start];

    let mut cursor = 0;
    while cursor < packet.len() {
        let packet_block: &mut [u8; AES_BLOCK_SIZE] = (&mut packet
            [cursor..cursor + AES_BLOCK_SIZE])
            .try_into()
            .unwrap();

        // Decrypt
        aes_peripheral.process(packet_block, esp_hal::aes::Mode::Decryption256, key_buf);

        // Generate new key from decrypted data
        let mut block_cursor = 0;
        for i in 0..AES_KEY_SIZE {
            key_buf[i] = key_buf[i] ^ packet_block[block_cursor];
            block_cursor += 1;
            if block_cursor > AES_BLOCK_SIZE {
                block_cursor = 0;
            }
        }

        // Increment cursor
        cursor += AES_BLOCK_SIZE;
    }

    return Ok(packet);
}
