use crate::client::Client;
use crate::manager::ClientKey;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_mtproto::transports::Transport;
use md5::Digest;
use std::sync::Arc;
use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::Mutex;

const PART_SIZE: usize = 512 * 1024;

pub struct State<S, H> {
    stream: S,
    next: Option<Vec<u8>>,
    file_part: i32,
    total_stream_size: usize,
    pub hasher: H,
    pub file_total_parts: i32,
}

impl<S, H> State<S, H> {
    pub fn new(stream: S, hasher: H) -> Self {
        Self {
            stream,
            next: None,
            file_part: 0,
            total_stream_size: 0,
            hasher,
            file_total_parts: 0,
        }
    }
}

pub async fn run_worker<T, S, R, H>(
    client: Client<T, S>,
    state: Arc<Mutex<State<R, H>>>,
    file_id: i64,
    conn_id: i32,
    big: bool,
) -> Result<(), Error>
where
    T: Transport + Send + 'static,
    S: Session,
    R: AsyncRead + Unpin,
    H: Digest,
{
    loop {
        let (file_part, file_total_parts, bytes, last) = {
            let mut state = state.lock().await;

            let Some(bytes) = state.next.take() else {
                state.next = Some(recv_buf(&mut state.stream).await?);
                continue;
            };

            if bytes.is_empty() {
                state.next = Some(Vec::new());
                break Ok(());
            }

            let next = recv_buf(&mut state.stream).await?;
            let last = next.is_empty();
            state.next = Some(next);

            let file_part = state.file_part;
            state.file_part += 1;

            state.total_stream_size += bytes.len();

            if !big {
                state.hasher.update(&bytes);
            }

            state.file_total_parts = if last {
                state.total_stream_size.div_ceil(PART_SIZE) as i32
            } else {
                -1
            };

            (file_part, state.file_total_parts, bytes, last)
        };

        let key = ClientKey::Upload { conn_id };

        if big {
            client.call_key(key, tl::functions::upload::SaveBigFilePart {
                file_id,
                file_part,
                file_total_parts,
                bytes,
            }).await?;
        } else {
            client.call_key(key, tl::functions::upload::SaveFilePart {
                file_id,
                file_part,
                bytes,
            }).await?;
        }

        if last {
            break Ok(());
        }
    }
}

async fn recv_buf<S>(stream: S) -> io::Result<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut bytes = vec![0; PART_SIZE];
    let part_size = fill_buf(stream, &mut bytes).await?;
    bytes.truncate(part_size);

    Ok(bytes)
}

async fn fill_buf<S>(mut stream: S, buf: &mut [u8]) -> io::Result<usize>
where
    S: AsyncRead + Unpin,
{
    let mut pos = 0;

    while pos < buf.len() {
        let n = stream.read(&mut buf[pos..]).await?;

        if n == 0 {
            break;
        }

        pos += n;
    }

    Ok(pos)
}
