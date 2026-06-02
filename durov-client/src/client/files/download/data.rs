use crate::client::files::download::extend::ExtendWriter;
use async_trait::async_trait;
use std::convert::identity;
use std::path::Path;
use tokio::fs::File;
use tokio::io;
use tokio::io::AsyncWrite;

#[async_trait(?Send)]
pub trait Download {
    type Stream: AsyncWrite;
    type Output;

    async fn into_stream(self) -> io::Result<(Self::Stream, fn(Self::Stream) -> Self::Output)>;
}

pub struct DownloadStream<S>(pub S)
where
    S: AsyncWrite;

pub struct DownloadBytes<B>(pub B)
where
    B: Extend<u8>;

pub struct DownloadFile<P>(pub P)
where
    P: AsRef<Path>;

#[async_trait(?Send)]
impl<S> Download for DownloadStream<S>
where
    S: AsyncWrite,
{
    type Stream = S;
    type Output = S;

    async fn into_stream(self) -> io::Result<(Self::Stream, fn(Self::Stream) -> Self::Output)> {
        Ok((self.0, identity))
    }
}

#[async_trait(?Send)]
impl<B> Download for DownloadBytes<B>
where
    B: Extend<u8>,
{
    type Stream = ExtendWriter<B>;
    type Output = B;

    async fn into_stream(self) -> io::Result<(Self::Stream, fn(Self::Stream) -> Self::Output)> {
        Ok((ExtendWriter(self.0), |w| w.0))
    }
}

#[async_trait(?Send)]
impl<P> Download for DownloadFile<P>
where
    P: AsRef<Path>,
{
    type Stream = File;
    type Output = ();

    async fn into_stream(self) -> io::Result<(Self::Stream, fn(Self::Stream) -> Self::Output)> {
        Ok((File::create(self.0).await?, drop))
    }
}
