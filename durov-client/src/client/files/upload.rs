pub mod data;
mod work;

use crate::client::files::join_futures;
use crate::client::files::upload::data::Upload;
use crate::client::files::upload::work::{run_worker, State};
use crate::client::Client;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_mtproto::transports::Transport;
use md5::{Digest, Md5};
use std::iter;
use std::sync::Arc;
use tokio::io::AsyncRead;
use tokio::sync::Mutex;

const CONNECTION_COUNT: usize = 8;
const WORKER_PER_CONNECTION: usize = 2;

impl<T: Transport, S: Session> Client<T, S>
where
    T: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    pub async fn upload_photo<D>(&self, data: D) -> Result<tl::enums::InputFile, Error>
    where
        D: Upload,
        D::Stream: Unpin + Send + 'static,
    {
        self.upload_file::<_, Md5>(data.into_stream().await?, false).await
    }

    pub async fn upload_document<D>(&self, data: D) -> Result<tl::enums::InputFile, Error>
    where
        D: Upload,
        D::Stream: Unpin + Send + 'static,
    {
        self.upload_file::<_, Md5>(data.into_stream().await?, true).await
    }

    async fn upload_file<R, H>(&self, stream: R, big: bool) -> Result<tl::enums::InputFile, Error>
    where
        R: AsyncRead + Unpin + Send + 'static,
        H: Digest + Send + 'static,
    {
        let file_id = rand::random();

        let state = State::new(stream, H::new());
        let state = Arc::new(Mutex::new(state));

        let iter = iter::zip(0..CONNECTION_COUNT, 0..WORKER_PER_CONNECTION)
            .map(|(conn_id, _)| run_worker(self.clone(), Arc::clone(&state), file_id, conn_id as i32, big));
        join_futures(iter).await?;

        let state = Arc::into_inner(state)
            .expect("all tasks should be joined")
            .into_inner();

        if big {
            Ok(tl::types::InputFileBig {
                id: file_id,
                parts: state.file_total_parts,
                name: String::new(),
            }.into())
        } else {
            let hash = state.hasher.finalize();
            let md5_checksum = hex::encode(hash);

            Ok(tl::types::InputFile {
                id: file_id,
                parts: state.file_total_parts,
                name: String::new(),
                md5_checksum,
            }.into())
        }
    }
}
