use durov_client::config::Config;
use durov_client::tl;

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
    let config = Config::new(api_id, api_hash);

    let client = Client::connect("userbot.session", config).await?;

    // If client is unauthorized:
    // client.interactive_login("ENTER YOUR PHONE").await?;

    let res = client.call(tl::functions::users::GetUsers {
        id: vec![tl::types::InputUserSelf {}.into()],
    }).await?;
    println!("{res:#?}");

    Ok(())
}
