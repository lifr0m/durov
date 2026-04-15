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
    pub prod_dc: bool,
    pub use_compression: bool,
    pub updates: bool,
    pub catch_up: bool,
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
            prod_dc: true,
            use_compression: true,
            updates: false,
            catch_up: false,
        }
    }
}
