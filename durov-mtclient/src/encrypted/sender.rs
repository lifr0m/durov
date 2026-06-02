use durov_tl_types::buffer::Buffer;
use std::collections::VecDeque;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;

pub struct Sender {
    writer: OwnedWriteHalf,
    pub bufs: VecDeque<Buffer>,
    pub pos: usize,
}

impl Sender {
    pub fn new(writer: OwnedWriteHalf) -> Self {
        Self {
            writer,
            bufs: VecDeque::new(),
            pos: 0,
        }
    }

    pub async fn send(&mut self) -> io::Result<usize> {
        let buf = &self.bufs[0];
        let slice = &buf[self.pos..];
        self.writer.write(slice).await
    }

    pub fn condition(&self) -> bool {
        !self.bufs.is_empty()
    }
}
