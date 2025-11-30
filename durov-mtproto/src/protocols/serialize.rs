use durov_tl_types::buffer::Buffer;
use durov_tl_types::serialize::Serialize;

pub fn serialize_len_first<T: Serialize>(buf: &mut Buffer, object: T) {
    0_i32.serialize(buf);
    let start = buf.len();
    object.serialize(buf);
    let len = (buf.len() - start) as i32;
    buf[start - 4..start].copy_from_slice(&len.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_len_first() {
        let object = durov_tl_types::schemas::api::functions::help::GetConfig {};
        let mut buf = Buffer::new();
        serialize_len_first(&mut buf, object);
        assert_eq!(buf[..], [4, 0, 0, 0, 0x6b, 0x18, 0xf9, 0xc4]);
    }
}
