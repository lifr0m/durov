pub mod data;
mod extend;
mod work;

use crate::client::files::download::data::Download;
use crate::client::files::download::work::{run_worker, State};
use crate::client::files::join_futures;
use crate::client::Client;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_mtproto::transports::Transport;
use std::sync::Arc;
use tokio::sync::Mutex;

const WORKER_COUNT: usize = 1;

impl<T: Transport, S: Session> Client<T, S>
where
    T: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    pub async fn download_file<D>(&self, data: D, location: tl::enums::InputFileLocation, dc_id: i32)
        -> Result<D::Output, Error>
    where
        D: Download,
        D::Stream: Unpin + Send + 'static,
    {
        let state = State::new();
        let state = Arc::new(Mutex::new(state));

        let (stream, back) = data.into_stream().await?;
        let stream = Arc::new(Mutex::new(stream));

        let iter = (0..WORKER_COUNT)
            .map(|_| run_worker(self.clone(), Arc::clone(&state), location.clone(), dc_id, Arc::clone(&stream)));
        join_futures(iter).await?;

        let stream = Arc::into_inner(stream)
            .expect("all tasks should be joined")
            .into_inner();

        Ok(back(stream))
    }
}
