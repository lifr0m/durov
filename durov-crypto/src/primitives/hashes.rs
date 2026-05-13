use crc_fast::CrcAlgorithm;
use sha1::Sha1;
use sha2::{Digest, Sha256};

pub fn crc32<const N: usize>(data: [&[u8]; N]) -> i32 {
    let mut hasher = crc_fast::Digest::new(CrcAlgorithm::Crc32IsoHdlc);
    for elem in data {
        hasher.update(elem);
    }
    hasher.finalize() as i32
}

pub fn sha256<const N: usize>(data: [&[u8]; N]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for elem in data {
        hasher.update(elem);
    }
    hasher.finalize().0
}

pub fn sha1<const N: usize>(data: [&[u8]; N]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    for elem in data {
        hasher.update(elem);
    }
    hasher.finalize().0
}
