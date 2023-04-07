use rand::{
    distributions::{Alphanumeric, DistString},
    Rng,
};
use trdelnik_client::{Keypair, Pubkey};

pub fn random_pubkey() -> Pubkey {
    Pubkey::new_unique()
}

pub fn random_keypair() -> Keypair {
    Keypair::new()
}

pub fn random_i64(lower: i64, upper: i64) -> i64 {
    rand::thread_rng().gen_range(lower..=upper)
}

pub fn random_u64(lower: u64, upper: u64) -> u64 {
    rand::thread_rng().gen_range(lower..=upper)
}

pub fn random_bool() -> bool {
    rand::thread_rng().gen()
}

pub fn random_string(length_min: u64, length_max: u64) -> String {
    Alphanumeric.sample_string(
        &mut rand::thread_rng(),
        random_u64(length_min, length_max) as usize,
    )
}

pub fn random_bytes(length_min: u64, length_max: u64) -> Vec<u8> {
    let length = random_u64(length_min, length_max);
    let mut bytes = vec![0u8; length as usize];
    rand::thread_rng().fill(&mut bytes[..]);
    bytes
}
