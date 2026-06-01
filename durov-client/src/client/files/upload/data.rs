use async_trait::async_trait;
use std::io::Cursor;
use std::path::Path;
use tokio::fs::File;
use tokio::io;
use tokio::io::AsyncRead;

#[async_trait(?Send)]
pub trait Upload<S>
where
    S: AsyncRead,
{
    async fn into_stream(self) -> io::Result<S>;
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
impl<S> Upload<S> for UploadStream<S>
where
    S: AsyncRead,
{
    async fn into_stream(self) -> io::Result<S> {
        Ok(self.0)
    }
}

#[async_trait(?Send)]
impl<B> Upload<Cursor<B>> for UploadBytes<B>
where
    B: AsRef<[u8]> + Unpin,
{
    async fn into_stream(self) -> io::Result<Cursor<B>> {
        Ok(Cursor::new(self.0))
    }
}

#[async_trait(?Send)]
impl<P> Upload<File> for UploadFile<P>
where
    P: AsRef<Path>,
{
    async fn into_stream(self) -> io::Result<File> {
        File::open(self.0).await
    }
}
