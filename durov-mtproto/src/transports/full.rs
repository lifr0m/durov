use crate::crypto;
use crate::log::debug_bytes;
use crate::transports::{Error, Transport};
use durov_tl_types::buffer::Buffer;

pub struct Full {
    send_seq: i32,
    recv_seq: i32,
}

impl Default for Full {
    fn default() -> Self {
        Self::new()
    }
}

impl Full {
    pub fn new() -> Self {
        Self {
            send_seq: 0,
            recv_seq: 0,
        }
    }
}

impl Full {
    const START: usize = 4 + 4;
    const END: usize = 4;
    const FULL: usize = Self::START + Self::END;
}

impl Transport for Full {
    fn pack(&mut self, buf: &mut Buffer) {
        let len = Self::FULL + buf.len();
        let len = (len as i32).to_le_bytes();
        let seq = self.send_seq.to_le_bytes();
        let crc = crypto::crc32([&len, &seq, buf])
            .to_le_bytes();

        buf.extend_front(&seq);
        buf.extend_front(&len);
        buf.extend_back(&crc);

        debug_bytes("transport [full] (pack)", buf);

        self.send_seq += 1;
    }

    fn unpack(&mut self, buf: &mut Buffer) -> Result<(), Error> {
        debug_bytes("transport [full] (unpack)", buf);

        if buf.len() < 4 {
            return Err(Error::MissingBytes(4 - buf.len()));
        }

        let len = i32::from_le_bytes(buf.array(0));

        if len < 0 {
            return Err(Error::Application(-len));
        }

        let len = len as usize;

        if len < Self::FULL {
            return Err(Error::LengthTooSmall {
                expected: Self::FULL,
                received: len,
            });
        }

        if buf.len() < len {
            return Err(Error::MissingBytes(len - buf.len()));
        }

        let seq = i32::from_le_bytes(buf.array(4));

        if seq != self.recv_seq {
            return Err(Error::SeqMismatch {
                expected: self.recv_seq,
                received: seq,
            });
        }

        let crc = i32::from_le_bytes(buf.array(len - 4));

        let calc_crc = crypto::crc32([
            &buf[..len - 4],
        ]);
        if calc_crc != crc {
            return Err(Error::CrcMismatch {
                expected: calc_crc,
                received: crc,
            });
        }

        buf.truncate_front(Self::START);
        buf.truncate_back(Self::END);

        self.recv_seq += 1;
        Ok(())
    }
}
