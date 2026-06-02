use crate::client::Client;
use crate::sessions::Session;
use crate::{tl, Error};
use durov_mtproto::transports::Transport;
use std::sync::Arc;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;

const LIMIT: usize = 1024 * 1024;

pub struct State {
    offset: i64,
    finished: bool,
}

impl State {
    pub fn new() -> Self {
        Self {
            offset: 0,
            finished: false,
        }
    }
}

pub async fn run_worker<T, S, R>(
    client: Client<T, S>,
    state: Arc<Mutex<State>>,
    location: tl::enums::InputFileLocation,
    dc_id: i32,
    stream: Arc<Mutex<R>>,
) -> Result<(), Error>
where
    T: Transport + Send + 'static,
    S: Session,
    R: AsyncWrite + Unpin,
{
    loop {
        let offset = {
            let mut state = state.lock().await;

            if state.finished {
                break Ok(());
            }

            let offset = state.offset;
            state.offset += LIMIT as i64;

            offset
        };

        let file = client.call_dc(dc_id, tl::functions::upload::GetFile {
            precise: false,
            cdn_supported: false,
            location: location.clone(),
            offset,
            limit: LIMIT as i32,
        }).await?;

        let file = match file {
            tl::enums::upload::File::File(file) => file,
            tl::enums::upload::File::FileCdnRedirect(_) => panic!("cdn is not supported"),
        };

        if file.bytes.len() < LIMIT {
            let mut state = state.lock().await;

            state.finished = true;
        }

        let mut stream = stream.lock().await;

        stream.write_all(&file.bytes).await?;
    }
}
