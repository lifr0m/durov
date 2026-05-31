use async_trait::async_trait;
use std::path::Path;
use tokio::fs::File;
use tokio::io;
use tokio::io::AsyncRead;

#[async_trait(?Send)]
pub trait Upload<S>
where
    S: AsyncRead,
{
    async fn stream(self) -> io::Result<S>;
}

#[async_trait(?Send)]
impl<S> Upload<S> for S
where
    S: AsyncRead,
{
    async fn stream(self) -> io::Result<S> {
        Ok(self)
    }
}

#[async_trait(?Send)]
impl Upload<File> for &Path {
    async fn stream(self) -> io::Result<File> {
        File::open(self).await
    }
}
