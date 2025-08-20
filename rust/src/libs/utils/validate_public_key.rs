use blst::min_pk::PublicKey;

pub fn validate_public_key(pk_bytes: &[u8]) -> bool {
    // Try to deserialize a public key from bytes
    PublicKey::from_bytes(pk_bytes).is_ok()
}
