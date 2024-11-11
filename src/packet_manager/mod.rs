pub mod commander;

pub static CLUSTER_SECRET: &'static [u8] = include_bytes!("../../keys/cluster_key.dat");
