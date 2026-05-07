use std::fmt::Write;

pub fn debug_bytes(comment: &str, data: &[u8]) {
    if !tracing::enabled!(tracing::Level::DEBUG) {
        return;
    }
    let mut string = comment.to_string();
    for (idx, &byte) in data.iter().enumerate() {
        if idx.is_multiple_of(0x10) {
            write!(&mut string, "\n{:04X} | ", idx)
                .unwrap();
        }
        write!(&mut string, "{byte:02X} ")
            .unwrap();
    }
    tracing::debug!("{string}");
}
