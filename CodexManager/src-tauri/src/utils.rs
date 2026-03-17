use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use once_cell::sync::Lazy;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub(crate) static HTTP: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

pub(crate) fn UnixNow() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

pub(crate) fn GeneratePkce() -> (String, String) {
    let mut Bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut Bytes);
    let Verifier = URL_SAFE_NO_PAD.encode(Bytes);
    let Challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(Verifier.as_bytes()));
    (Verifier, Challenge)
}

pub(crate) fn GenerateState() -> String {
    let mut Bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut Bytes);
    URL_SAFE_NO_PAD.encode(Bytes)
}
