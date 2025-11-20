use ige::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyIvInit};

pub fn aes256_ige_encrypt(msg: &mut [u8], key: [u8; 32], iv: [u8; 32]) {
    let mut encryptor = ige::Encryptor::<aes::Aes256>::new(
        (&key).into(),
        (&iv).into(),
    );
    for chunk in msg.chunks_mut(16) {
        let block = chunk.try_into()
            .unwrap();
        encryptor.encrypt_block(block);
    }
}

pub fn aes256_ige_decrypt(msg: &mut [u8], key: [u8; 32], iv: [u8; 32]) {
    let mut decryptor = ige::Decryptor::<aes::Aes256>::new(
        (&key).into(),
        (&iv).into(),
    );
    for chunk in msg.chunks_mut(16) {
        let block = chunk.try_into()
            .unwrap();
        decryptor.decrypt_block(block);
    }
}
