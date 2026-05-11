use bytes::BufMut;
use durov_tl_types::buffer::Buffer;
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;

pub struct Receiver {
    reader: OwnedReadHalf,
    pub buf: Buffer,
    pub limit: usize,
}

impl Receiver {
    pub fn new(reader: OwnedReadHalf) -> Self {
        Self {
            reader,
            buf: Buffer::new(),
            limit: 0,
        }
    }

    pub async fn recv(&mut self) -> io::Result<usize> {
        let mut limit = (&mut self.buf).limit(self.limit);
        self.reader.read_buf(&mut limit).await
    }
}
