use std::fmt::Write;

pub fn debug_bytes<const N: usize>(comment: &str, data: [&[u8]; N]) {
    if !log::log_enabled!(log::Level::Debug) {
        return;
    }
    let mut string = String::from(comment);
    for (idx, &byte) in data.into_iter()
        .flatten()
        .enumerate()
    {
        if idx.is_multiple_of(0x10) {
            write!(&mut string, "\n{:04X} | ", idx)
                .unwrap();
        }
        write!(&mut string, "{byte:02X} ")
            .unwrap();
    }
    log::debug!("{string}");
}
