use durov_tl_types::buffer::Buffer;
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;

pub struct Receiver {
    reader: OwnedReadHalf,
    pub buf: Buffer,
    pub pos: usize,
}

impl Receiver {
    pub fn new(reader: OwnedReadHalf) -> Self {
        Self {
            reader,
            buf: Buffer::new(),
            pos: 0,
        }
    }

    pub async fn select(&mut self) -> io::Result<usize> {
        let slice = &mut self.buf[self.pos..];
        self.reader.read(slice).await
    }
}
