pub struct Auth {
    pub dc_id: i32,
    pub dc_host: String,
    pub dc_port: u16,
    pub auth_key: [u8; 256],
    pub main: bool,
}
