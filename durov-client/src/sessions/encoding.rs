#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PeerType {
    User,
    Chat,
    Channel,
}

pub fn encode_peer_id(id: i64, typ: PeerType) -> i64 {
    match typ {
        PeerType::User => id,
        PeerType::Chat => -id,
        PeerType::Channel => -(id + 1000000000000),
    }
}

pub fn decode_peer_id(id: i64) -> (i64, PeerType) {
    match id {
        0 => panic!("tried decoding peer id 0"),
        1.. => (id, PeerType::User),
        -1000000000000.. => (-id, PeerType::Chat),
        _ => (-id - 1000000000000, PeerType::Channel),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_peer_id() {
        assert_eq!(encode_peer_id(42, PeerType::User), 42);
        assert_eq!(encode_peer_id(42, PeerType::Chat), -42);
        assert_eq!(encode_peer_id(42, PeerType::Channel), -1000000000042);
    }

    #[test]
    fn test_decode_peer_id() {
        assert_eq!(decode_peer_id(42), (42, PeerType::User));
        assert_eq!(decode_peer_id(-42), (42, PeerType::Chat));
        assert_eq!(decode_peer_id(-1000000000042), (42, PeerType::Channel));
    }
}
