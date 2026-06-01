pub mod upload;

use tokio::task::JoinSet;

async fn join_futures<I, T, E>(iter: I) -> Result<Vec<T>, E>
where
    I: IntoIterator,
    I::Item: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    let mut set = iter.into_iter().collect::<JoinSet<_>>();
    let mut results = Vec::with_capacity(set.len());
    while let Some(res) = set.join_next().await {
        results.push(res.unwrap()?);
    }
    Ok(results)
}
