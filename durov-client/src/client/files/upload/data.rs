use async_trait::async_trait;
use std::io::Cursor;
use std::path::Path;
use tokio::fs::File;
use tokio::io;
use tokio::io::AsyncRead;

#[async_trait(?Send)]
pub trait Upload {
    type Stream: AsyncRead;

    async fn into_stream(self) -> io::Result<Self::Stream>;
}

pub struct UploadStream<S>(pub S)
where
    S: AsyncRead;

pub struct UploadBytes<B>(pub B)
where
    B: AsRef<[u8]>;

pub struct UploadFile<P>(pub P)
where
    P: AsRef<Path>;

#[async_trait(?Send)]
impl<S> Upload for UploadStream<S>
where
    S: AsyncRead,
{
    type Stream = S;

    async fn into_stream(self) -> io::Result<Self::Stream> {
        Ok(self.0)
    }
}

#[async_trait(?Send)]
impl<B> Upload for UploadBytes<B>
where
    B: AsRef<[u8]> + Unpin,
{
    type Stream = Cursor<B>;

    async fn into_stream(self) -> io::Result<Self::Stream> {
        Ok(Cursor::new(self.0))
    }
}

#[async_trait(?Send)]
impl<P> Upload for UploadFile<P>
where
    P: AsRef<Path>,
{
    type Stream = File;

    async fn into_stream(self) -> io::Result<Self::Stream> {
        File::open(self.0).await
    }
}
