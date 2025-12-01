use durov_tl_types::buffer::Buffer;
use durov_tl_types::serialize::Serialize;

pub fn serialize_len_first<F>(buf: &mut Buffer, f: F)
where
    F: Fn(&mut Buffer),
{
    0_i32.serialize(buf);
    let start = buf.len();
    f(buf);
    let len = (buf.len() - start) as i32;
    buf[start - 4..start].copy_from_slice(&len.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_len_first() {
        let mut buf = Buffer::new();
        buf.extend_back(&[1, 2]);
        let object = durov_tl_types::schemas::api::functions::help::GetConfig {};
        serialize_len_first(&mut buf, |buf| object.serialize(buf));
        assert_eq!(buf[..], [1, 2, 4, 0, 0, 0, 0x6b, 0x18, 0xf9, 0xc4]);
    }
}
