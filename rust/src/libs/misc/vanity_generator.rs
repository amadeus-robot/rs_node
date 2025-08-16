use rand::RngCore;
use rayon::prelude::*;
use bs58::encode as bs58_encode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// Dummy placeholder for BlsEx.get_public_key!(sk)
fn bls_get_public_key(sk: &[u8]) -> Vec<u8> {
    // Replace this with your actual BLS public key computation
    let mut buf = vec![0u8; 48];
    buf.copy_from_slice(&sk[..48.min(sk.len())]);
    buf
}

pub struct VanityGenerator;

impl VanityGenerator {
    pub fn parallel(
        prefix: &str,
        skip_leading: bool,
        case_insensitive: bool,
        max_tries: usize,
    ) -> Option<(String, String)> {
        let found = Arc::new(AtomicBool::new(false));

        (0..max_tries).into_par_iter()
            .find_map_any(|_| {
                if found.load(Ordering::Relaxed) { return None; }
                if let Some(res) = Self::go(prefix, skip_leading, case_insensitive, max_tries) {
                    found.store(true, Ordering::Relaxed);
                    Some(res)
                } else {
                    None
                }
            })
    }

    pub fn go(
        prefix: &str,
        skip_leading: bool,
        case_insensitive: bool,
        max_tries: usize,
    ) -> Option<(String, String)> {
        for _ in 0..max_tries {
            // Generate strong random 64 bytes
            let mut sk = [0u8; 64];
            rand::thread_rng().fill_bytes(&mut sk);

            // Compute public key
            let full_key_bytes = bls_get_public_key(&sk);
            let full_key = bs58_encode(full_key_bytes).into_string();

            // Skip leading byte if needed
            let key = if skip_leading {
                full_key.chars().skip(1).collect::<String>()
            } else {
                full_key.clone()
            };

            // Get the slice to compare with prefix
            let key_slice = &key[..prefix.len().min(key.len())];

            // Check prefix
            if case_insensitive {
                if key_slice.eq_ignore_ascii_case(prefix) {
                    return Some((full_key, bs58_encode(sk).into_string()));
                }
            } else if key_slice == prefix {
                return Some((full_key, bs58_encode(sk).into_string()));
            }
        }
        None
    }
}
