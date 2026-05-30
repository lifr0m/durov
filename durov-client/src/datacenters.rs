use durov_crypto::Datacenter;

pub const PUBLIC_KEY: &str = "-----BEGIN RSA PUBLIC KEY-----
MIIBCgKCAQEA6LszBcC1LGzyr992NzE0ieY+BSaOW622Aa9Bd4ZHLl+TuFQ4lo4g
5nKaMBwK/BIb9xUfg0Q29/2mgIR6Zr9krM7HjuIcCzFvDtr+L0GQjae9H0pRB2OO
62cECs5HKhT5DZ98K33vmWiLowc621dQuwKWSQKjWf50XYFw42h21P2KXUGyp2y/
+aEyZ+uVgLLQbRA1dEjSDZ2iGRy12Mk5gpYc397aYp438fsJoHIgJ2lgMv5h7WY9
t6N/byY9Nw9p21Og3AoXSL2q/2IJ1WRUhebgAdGVMlV1fkuOQoEzR7EdpqtQD9Cs
5+bfo3Nhmcyvk5ftB0WkJ9z6bNZ7yxrP8wIDAQAB
-----END RSA PUBLIC KEY-----";

pub fn static_dc(id: i32) -> Datacenter {
    let (host, port) = match id {
        1 => ("149.154.175.53", 443),
        2 => ("149.154.167.41", 443),
        3 => ("149.154.175.100", 443),
        4 => ("149.154.167.91", 443),
        5 => ("91.108.56.155", 443),
        _ => panic!("invalid dc id: {id}"),
    };
    Datacenter {
        id,
        host: host.to_string(),
        port,
        pubkey: PUBLIC_KEY,
    }
}
