use crate::tl;

pub struct Config {
    pub api_id: i32,
    pub api_hash: String,
    pub device_model: String,
    pub system_version: String,
    pub app_version: String,
    pub system_lang_code: String,
    pub lang_pack: String,
    pub lang_code: String,
    pub params: Option<tl::enums::JsonValue>,
    pub use_compression: bool,
}

impl Config {
    pub fn new(api_id: i32, api_hash: String) -> Self {
        Self {
            api_id,
            api_hash,
            device_model: String::from("Unknown"),
            system_version: String::from("Unknown"),
            app_version: String::from("Unknown"),
            system_lang_code: String::from("en"),
            lang_pack: String::new(),
            lang_code: String::from("en"),
            params: None,
            use_compression: true,
        }
    }
}
