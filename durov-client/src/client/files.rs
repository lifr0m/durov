pub mod upload;

use crate::client::files::upload::Upload;
use crate::client::Client;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_mtproto::transports::Transport;
use md5::{Digest, Md5};
use tokio::io::{AsyncRead, AsyncReadExt};

const PART_SIZE: usize = 512 * 1024;
const UNKNOWN_TOTAL_PARTS: i32 = -1;

impl<T: Transport, S: Session> Client<T, S>
where
    T: Send + 'static,
{
    pub async fn upload_photo<D, R>(&self, data: D) -> Result<tl::enums::InputFile, Error>
    where
        D: Upload<R>,
        R: AsyncRead + Unpin,
    {
        let save_part = async |file_id, file_part, _file_total_parts, bytes| {
            self.call(tl::functions::upload::SaveFilePart {
                file_id,
                file_part,
                bytes,
            }).await?;

            Ok(())
        };

        let finalize = |file_id, file_total_parts, hasher: Md5| {
            let hash = hasher.finalize();
            let md5_checksum = hex::encode(hash);

            tl::types::InputFile {
                id: file_id,
                parts: file_total_parts,
                name: String::new(),
                md5_checksum,
            }.into()
        };

        upload_file(data.stream().await?, save_part, finalize).await
    }

    pub async fn upload_document<D, R>(&self, data: D) -> Result<tl::enums::InputFile, Error>
    where
        D: Upload<R>,
        R: AsyncRead + Unpin,
    {
        let save_part = async |file_id, file_part, file_total_parts, bytes| {
            self.call(tl::functions::upload::SaveBigFilePart {
                file_id,
                file_part,
                file_total_parts,
                bytes,
            }).await?;

            Ok(())
        };

        let finalize = |file_id, file_total_parts, _hasher: Md5| {
            tl::types::InputFileBig {
                id: file_id,
                parts: file_total_parts,
                name: String::new(),
            }.into()
        };

        upload_file(data.stream().await?, save_part, finalize).await
    }
}

async fn upload_file<H, S, P, F>(mut stream: S, mut save_part: P, finalize: F)
    -> Result<tl::enums::InputFile, Error>
where
    H: Digest,
    S: AsyncRead + Unpin,
    P: AsyncFnMut(i64, i32, i32, Vec<u8>) -> Result<(), Error>,
    F: FnOnce(i64, i32, H) -> tl::enums::InputFile,
{
    let file_id = rand::random();
    let mut file_part = 0;
    let mut total_stream_size = 0;

    let mut hasher = H::new();

    let file_total_parts = loop {
        let mut bytes = vec![0; PART_SIZE];
        let part_size = fill_buf(&mut stream, &mut bytes).await?;
        bytes.truncate(part_size);

        total_stream_size += part_size;
        hasher.update(&bytes);

        let file_total_parts = calculate_file_total_parts(part_size, total_stream_size);

        save_part(file_id, file_part, file_total_parts, bytes).await?;

        if file_total_parts != UNKNOWN_TOTAL_PARTS {
            break file_total_parts;
        }

        file_part += 1;
    };

    Ok(finalize(file_id, file_total_parts, hasher))
}

async fn fill_buf<S>(mut stream: S, buf: &mut [u8]) -> Result<usize, Error>
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

fn calculate_file_total_parts(part_size: usize, total_stream_size: usize) -> i32 {
    if part_size < PART_SIZE {
        total_stream_size.div_ceil(PART_SIZE) as i32
    } else {
        UNKNOWN_TOTAL_PARTS
    }
}
