use durov_client::client::files::download::data::{DownloadBytes, DownloadFile, DownloadStream};
use durov_client::client::files::upload::data::{UploadBytes, UploadFile, UploadStream};
use durov_client::config::Config;
use durov_client::tl;
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

    // --- UPLOAD ---

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

    // --- DOWNLOAD ---

    let location = tl::enums::InputFileLocation::InputPhotoFileLocation(
        tl::types::InputPhotoFileLocation {
            id: 0,
            access_hash: 0,
            file_reference: Vec::new(),
            thumb_size: String::new(),
        }
    );
    let dc_id = 3;

    // stream
    let stream = File::create("file.txt").await?;
    let stream = client.download_file(DownloadStream(stream), location.clone(), dc_id).await?;

    // bytes in memory
    let bytes = Vec::new();
    let bytes = client.download_file(DownloadBytes(bytes), location.clone(), dc_id).await?;

    // file path
    let path = "file.txt";
    client.download_file(DownloadFile(path), location.clone(), dc_id).await?;

    Ok(())
}
