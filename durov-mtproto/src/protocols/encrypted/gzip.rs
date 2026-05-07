use durov_tl_types::deserialize;
use flate2::bufread::{GzDecoder, GzEncoder};
use flate2::Compression;
use std::io::Read;

pub fn gzip_encode(input: &[u8]) -> Vec<u8> {
    let level = Compression::default();
    let mut encoder = GzEncoder::new(input, level);
    let mut output = Vec::new();
    encoder.read_to_end(&mut output)
        .unwrap();
    output
}

pub fn gzip_decode(input: &[u8]) -> Result<Vec<u8>, deserialize::Error> {
    let mut decoder = GzDecoder::new(input);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)
        .map_err(deserialize::Error::GzipDecode)?;
    Ok(output)
}
