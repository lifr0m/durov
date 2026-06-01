use durov_client::client::files::upload::data::{UploadBytes, UploadFile, UploadStream};
use durov_client::config::Config;
use tokio::fs::File;

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

    let client = Client::connect("files.session", config).await?;

    // PHOTO (compressed on telegram side)
    // If your photo is larger or equal 10 MB, upload as document

    // stream
    let stream = File::open("photo.png").await?;
    let input_file = client.upload_photo(UploadStream(stream)).await?;

    // bytes in memory
    let bytes = vec![0x41, 0x42, 0x43];
    let input_file = client.upload_photo(UploadBytes(bytes)).await?;

    // file path
    let path = "photo.png";
    let input_file = client.upload_photo(UploadFile(path)).await?;

    // DOCUMENT (or photo as document)

    // stream
    let stream = File::open("document.pdf").await?;
    let input_file = client.upload_document(UploadStream(stream)).await?;

    // bytes in memory
    let bytes = vec![0x41, 0x42, 0x43];
    let input_file = client.upload_document(UploadBytes(bytes)).await?;

    // file path
    let path = "document.pdf";
    let input_file = client.upload_document(UploadFile(path)).await?;

    Ok(())
}
