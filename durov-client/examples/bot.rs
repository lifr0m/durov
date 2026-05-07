use durov_client::config::Config;

type Client = durov_client::client::Client<
    durov_mtproto::transports::full::Full,
    durov_client::sessions::telethon::Telethon,
>;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .init();

    let api_id = 0; // ENTER YOUR API ID
    let api_hash = String::from("ENTER YOUR API HASH");
    let mut config = Config::new(api_id, api_hash);
    config.updates = true;
    // Uncomment if you want to pull updates which occurred while bot was offline.
    // config.catch_up = true;

    let client = Client::connect("bot.session", config).await?;

    // If client is unauthorized:
    // client.bot_login("ENTER YOUR TOKEN").await?;

    // You can use `client.next_unauthorized_updates` to receive updates
    // which happen on unauthorized connections, for example QR login.

    loop {
        tokio::select! {
            // Rust doesn't have `AsyncDrop` trait (yet) so we have to manually save updates state.
            _ = tokio::signal::ctrl_c() => {
                client.save_updates().await?;
                break;
            }
            updates = client.next_authorized_updates() => {
                for update in updates? {
                    println!("{update:?}");
                }
            }
        }
    }

    Ok(())
}
