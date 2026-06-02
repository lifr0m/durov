use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io;
use tokio::io::AsyncWrite;

pub struct ExtendWriter<T>(pub T);

impl<T> Unpin for ExtendWriter<T> {}

impl<T> AsyncWrite for ExtendWriter<T>
where
    T: Extend<u8>,
{
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.0.extend(buf.iter().copied());
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
