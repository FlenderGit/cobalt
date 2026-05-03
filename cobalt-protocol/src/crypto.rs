use std::io;

use num_bigint::BigInt;
use openssl::{
    pkey::Private,
    rand::rand_bytes,
    rsa::{Padding, Rsa},
    sha::sha1,
    symm::{Cipher, Crypter, Mode},
};

pub fn rsa_decrypt(private_key: &Rsa<Private>, encrypted: &[u8]) -> io::Result<Vec<u8>> {
    let mut decrypted = vec![0u8; private_key.size() as usize];
    let len = private_key
        .private_decrypt(encrypted, &mut decrypted, Padding::PKCS1)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    decrypted.truncate(len);
    Ok(decrypted)
}

#[derive(Debug)]
pub struct CryptoConfig {
    pub private_key: Rsa<Private>,
    pub public_key_der: Vec<u8>,
}

impl CryptoConfig {
    pub fn new(public_key_der: &[u8], private_key_pem: &[u8]) -> io::Result<Self> {
        let private_key = Rsa::private_key_from_pem(private_key_pem)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(Self {
            private_key,
            public_key_der: public_key_der.to_vec(),
        })
    }
}

pub struct SessionCrypto {
    pub encryptor: Cfb8Encryptor,
    pub decryptor: Cfb8Decryptor,
}

impl SessionCrypto {
    pub fn new(shared_secret: &[u8; 16]) -> io::Result<Self> {
        Ok(Self {
            encryptor: Cfb8Encryptor::new(shared_secret)?,
            decryptor: Cfb8Decryptor::new(shared_secret)?,
        })
    }
}

pub struct Cfb8Encryptor {
    crypter: Crypter,
}

impl Cfb8Encryptor {
    /// Create a new encryptor with the given 16-byte key.
    /// The IV is the same as the key, as required by Minecraft.
    pub fn new(key: &[u8; 16]) -> io::Result<Self> {
        let mut crypter = Crypter::new(Cipher::aes_128_cfb8(), Mode::Encrypt, key, Some(key))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        crypter.pad(false);
        Ok(Self { crypter })
    }

    /// Encrypt a slice of plaintext bytes, returning the ciphertext.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> io::Result<Vec<u8>> {
        let mut output = vec![0u8; plaintext.len() + 16];
        let n = self
            .crypter
            .update(plaintext, &mut output)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        output.truncate(n);
        Ok(output)
    }
}

pub struct Cfb8Decryptor {
    crypter: Crypter,
}

impl Cfb8Decryptor {
    /// Create a new decryptor with the given 16-byte key.
    /// The IV is the same as the key, as required by Minecraft.
    pub fn new(key: &[u8; 16]) -> io::Result<Self> {
        let mut crypter = Crypter::new(Cipher::aes_128_cfb8(), Mode::Decrypt, key, Some(key))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        crypter.pad(false);
        Ok(Self { crypter })
    }

    /// Decrypt a slice of ciphertext bytes, returning the plaintext.
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> io::Result<Vec<u8>> {
        let mut output = vec![0u8; ciphertext.len() + 16];
        let n = self
            .crypter
            .update(ciphertext, &mut output)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        output.truncate(n);
        Ok(output)
    }
}

pub fn generate_verify_token() -> [u8; 4] {
    let mut token = [0u8; 4];
    rand_bytes(&mut token).expect("Failed to generate random bytes");
    token
}

pub fn minecraft_hash(shared_secret: &[u8], public_key_der: &[u8]) -> String {
    let mut data = Vec::new();
    data.extend_from_slice(b"");
    data.extend_from_slice(shared_secret);
    data.extend_from_slice(public_key_der);

    let digest = sha1(&data);
    let bigint = BigInt::from_signed_bytes_be(&digest);
    format!("{:x}", bigint)
}
