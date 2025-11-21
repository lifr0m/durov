use super::{Error, Transport};
use crate::crypto;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub struct Full<S> {
    stream: S,
    send_seq: i32,
    recv_seq: i32,
}

impl<S> Full<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            send_seq: 0,
            recv_seq: 0,
        }
    }
}

#[async_trait]
impl<S: AsyncWrite + AsyncRead + Unpin + Send> Transport for Full<S> {
    async fn send(&mut self, payload: &[u8]) -> Result<(), Error> {
        let len = Self::OVERHEAD + payload.len();
        let len = (len as i32).to_le_bytes();
        let seq = self.send_seq.to_le_bytes();
        let crc = crypto::crc32([&len, &seq, payload])
            .to_le_bytes();

        self.stream.write_all(&len).await?;
        self.stream.write_all(&seq).await?;
        self.stream.write_all(payload).await?;
        self.stream.write_all(&crc).await?;

        self.send_seq += 1;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Vec<u8>, Error> {
        let len = self.stream.read_i32_le().await?;
        if len < 0 {
            return Err(Error::Application(-len));
        }
        let seq = self.stream.read_i32_le().await?;
        let mut payload = vec![0; len as usize - Self::OVERHEAD];
        self.stream.read_exact(&mut payload).await?;
        let crc = self.stream.read_i32_le().await?;
        let my_crc = crypto::crc32([&len.to_le_bytes(), &seq.to_le_bytes(), &payload]);

        if seq != self.recv_seq {
            return Err(Error::SeqMismatch {
                expected: self.recv_seq,
                received: seq,
            });
        }
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

impl<S> Full<S> {
    const OVERHEAD: usize = 4 + 4 + 4;
}
