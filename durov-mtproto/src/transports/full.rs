use super::{Error, Transport};
use crc_fast::{CrcAlgorithm, Digest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net;

pub struct Full {
    stream: net::TcpStream,
    send_seq: i32,
    recv_seq: i32,
}

impl Full {
    pub fn new(stream: net::TcpStream) -> Self {
        Self {
            stream,
            send_seq: 0,
            recv_seq: 0,
        }
    }
}

impl Transport for Full {
    async fn send(&mut self, payload: &[u8]) -> Result<(), Error> {
        let len = (payload.len() as i32).to_le_bytes();
        let seq = self.send_seq.to_le_bytes();

        let mut crc = Digest::new(CrcAlgorithm::Crc32IsoHdlc);
        crc.update(&len);
        crc.update(&seq);
        crc.update(payload);
        let crc = (crc.finalize() as i32).to_le_bytes();

        let len = payload.len() + 12;
        let len = (len as i32).to_le_bytes();

        self.stream.write_all(&len).await?;
        self.stream.write_all(&seq).await?;
        self.stream.write_all(payload).await?;
        self.stream.write_all(&crc).await?;

        self.send_seq += 1;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>, Error> {
        let len = self.stream.read_i32_le().await?;
        let len = (len - 12) as usize;

        let seq = self.stream.read_i32_le().await?;

        let mut payload = vec![0; len];
        self.stream.read_exact(&mut payload).await?;

        let crc = self.stream.read_i32_le().await?;

        if seq != self.recv_seq {
            return Err(Error::SeqMismatch {
                expected: self.recv_seq,
                received: seq,
            });
        }

        let mut my_crc = Digest::new(CrcAlgorithm::Crc32IsoHdlc);
        my_crc.update(&(len as i32).to_le_bytes());
        my_crc.update(&seq.to_le_bytes());
        my_crc.update(&payload);
        let my_crc = my_crc.finalize() as i32;

        if my_crc != crc {
            return Err(Error::CrcMismatch {
                expected: my_crc,
                received: crc,
            });
        }

        self.recv_seq += 1;
        Ok(payload)
    }
}
