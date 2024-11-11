//! This module implements a hardware-accelerated HMAC implmentation for packet-level verification.
//!
//! Huge thanks to this article for helping me figure this out.
//! https://medium.com/@short_sparrow/how-hmac-works-step-by-step-explanation-with-examples-f4aff5efb40e
//!
//! The existing HMAC implementation on crates.io has far too many abstractions to make
//! sense of it, and doesn't support hardware acceleration on the ESP32 by default.
//! That is likely going to be an issue since I want to HMAC every packet on esp-now

use esp_hal::sha::{Sha, Sha256};

const BLOCK_SIZE: usize = 64;
pub const HASH_SIZE: usize = 32;
const IPAD: u8 = 0x36;
const OPAD: u8 = 0x5C;

/// Verifies the authenticity of a packet using the cluster key
pub fn authenticate_packet<'a>(sha_peripheral: &mut Sha<'a>, packet: &'a [u8]) -> Option<&'a [u8]> {
    let hmac = &packet[0..HASH_SIZE];
    let data = &packet[HASH_SIZE..];

    let hash = hmac_cluster_chunk(sha_peripheral, data);
    for i in 0..HASH_SIZE {
        if hmac[i] != hash[i] {
            return None;
        }
    }
    return Some(data);
}

pub fn hmac_chunk(hash_peripheral: &mut Sha<'_>, key: &[u8], chunk: &[u8]) -> [u8; HASH_SIZE] {
    let mut hasher = hash_peripheral.start::<Sha256>();
    let mut ipad_key = [0u8; BLOCK_SIZE];
    for (i, byte) in key.iter().enumerate() {
        if i > BLOCK_SIZE {
            // Not sure this is the right approach. May want to
            // revisit later.
            break;
        }
        ipad_key[i] = *byte ^ IPAD;
    }
    if let Err(err) = hasher.update(&ipad_key) {
        log::error!("Failed to update hash: {err:?}");
    }

    if let Err(err) = hasher.update(chunk) {
        log::error!("Failed to update hash: {err:?}");
    }

    let mut initial_hash = [0u8; HASH_SIZE];
    if let Err(err) = hasher.finish(&mut initial_hash) {
        log::error!("Failed to update hash: {err:?}");
    }
    drop(hasher);

    let mut hasher = hash_peripheral.start::<Sha256>();
    let mut opad_key = [0u8; BLOCK_SIZE];
    for (i, byte) in key.iter().enumerate() {
        if i > BLOCK_SIZE {
            break;
        }
        opad_key[i] = *byte ^ OPAD;
    }
    if let Err(err) = hasher.update(&opad_key) {
        log::error!("Failed to update hash: {err:?}");
    }
    if let Err(err) = hasher.update(&initial_hash) {
        log::error!("Failed to update hash: {err:?}");
    }

    let mut final_hmac = [0u8; HASH_SIZE];
    if let Err(err) = hasher.finish(&mut final_hmac) {
        log::error!("Failed to update hash: {err:?}");
    }

    return final_hmac;
}

pub static CLUSTER_KEY_IPAD: &[u8] = include_bytes!("../keys/cluster_key_ipad.dat");
pub static CLUSTER_KEY_OPAD: &[u8] = include_bytes!("../keys/cluster_key_opad.dat");

/// Accelerated form of `hmac_chunk` using pre-computed ipad
/// and opad values for the cluster key
pub fn hmac_cluster_chunk(hash_peripheral: &mut Sha<'_>, chunk: &[u8]) -> [u8; HASH_SIZE] {
    let mut hasher = hash_peripheral.start::<Sha256>();
    if let Err(err) = hasher.update(CLUSTER_KEY_IPAD) {
        log::error!("Failed to update hash: {err:?}");
    }

    if let Err(err) = hasher.update(chunk) {
        log::error!("Failed to update hash: {err:?}");
    }

    let mut initial_hash = [0u8; HASH_SIZE];
    if let Err(err) = hasher.finish(&mut initial_hash) {
        log::error!("Failed to update hash: {err:?}");
    }
    drop(hasher);

    let mut hasher = hash_peripheral.start::<Sha256>();
    if let Err(err) = hasher.update(CLUSTER_KEY_OPAD) {
        log::error!("Failed to update hash: {err:?}");
    }
    if let Err(err) = hasher.update(&initial_hash) {
        log::error!("Failed to update hash: {err:?}");
    }

    let mut final_hmac = [0u8; HASH_SIZE];
    if let Err(err) = hasher.finish(&mut final_hmac) {
        log::error!("Failed to update hash: {err:?}");
    }

    return final_hmac;
}
