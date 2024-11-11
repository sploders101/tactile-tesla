use rand::RngCore;
use std::{
    fs::File,
    io::{ErrorKind, Read, Write},
};

const BLOCK_SIZE: usize = 64;
pub const HASH_SIZE: usize = 32;
const IPAD: u8 = 0x36;
const OPAD: u8 = 0x5C;

fn main() {
    println!("cargo:rustc-link-arg-bins=-Tlinkall.x");
    println!("cargo:rustc-link-arg-bins=-Trom_functions.x");
    println!("cargo::rerun-if-changed=build.rs");

    // Create cluster ID
    if !std::fs::exists("keys/cluster_id.dat").unwrap() {
        let mut cluster_id = [0u8; 6];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut cluster_id);

        let mut file = File::create_new("keys/cluster_id.dat").unwrap();
        file.write_all(&cluster_id).unwrap();
    }

    // Create cluster secret
    println!("cargo::rerun-if-changed=keys/cluster_key.dat");
    match std::fs::File::create_new("keys/cluster_key.dat") {
        Ok(mut file) => {
            let mut cluster_id = [0u8; BLOCK_SIZE];
            let mut rng = rand::thread_rng();
            rng.fill_bytes(&mut cluster_id);

            file.write_all(&cluster_id).unwrap();
        }
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
        Err(err) => panic!("{err:?}"),
    }

    println!("cargo::rerun-if-changed=keys/cluster_key_ipad.dat");
    match std::fs::File::create_new("keys/cluster_key_ipad.dat") {
        Ok(mut file) => {
            let mut key = Vec::new();
            std::fs::File::open("keys/cluster_key.dat")
                .expect("Unable to read cluster key")
                .read_to_end(&mut key)
                .expect("Unable to read cluster key");
            let mut ipad_key = [0u8; BLOCK_SIZE];
            for (i, byte) in key.iter().enumerate() {
                if i > BLOCK_SIZE {
                    // Not sure this is the right approach. May want to
                    // revisit later.
                    break;
                }
                ipad_key[i] = *byte ^ IPAD;
            }
            file.write_all(&ipad_key)
                .expect("Unable to write cluster key ipad.");
        }
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
        Err(err) => panic!("{err:?}"),
    }

    println!("cargo::rerun-if-changed=keys/cluster_key_opad.dat");
    match std::fs::File::create_new("keys/cluster_key_opad.dat") {
        Ok(mut file) => {
            let mut key = Vec::new();
            std::fs::File::open("keys/cluster_key.dat")
                .expect("Unable to read cluster key")
                .read_to_end(&mut key)
                .expect("Unable to read cluster key");
            let mut opad_key = [0u8; BLOCK_SIZE];
            for (i, byte) in key.iter().enumerate() {
                if i > BLOCK_SIZE {
                    break;
                }
                opad_key[i] = *byte ^ OPAD;
            }
            file.write_all(&opad_key)
                .expect("Unable to write cluster key ipad.");
        }
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
        Err(err) => panic!("{err:?}"),
    }
}
