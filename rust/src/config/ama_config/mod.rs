use crate::*;
use blst::min_pk::SecretKey;
use once_cell::sync::Lazy;
use rand::{RngCore, rngs::OsRng};
use serde::Deserialize;
use std::{net::Ipv4Addr, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")] // maps "default", "trainer" from TOML
pub enum ComputorType {
    Trainer,
    Default,
    None,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SeedAnr {
    pub ip4: String,
    pub port: u16,
    pub version: String,
    pub signature: Vec<u8>,
    pub ts: u64,
    pub pk: Vec<u8>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AmaConfig {
    pub version: String,
    pub entry_size: usize,
    pub tx_size: usize,
    pub attestation_size: usize,
    pub quorum: u8,
    pub work_folder: PathBuf,
    pub offline: bool,
    pub http_ipv4: Ipv4Addr,
    pub http_port: u16,
    pub udp_ipv4_tuple: Ipv4Addr,
    pub udp_port: u16,
    pub seednodes: Vec<String>,
    pub seedanrs: Vec<SeedAnr>,
    pub othernodes: Vec<String>,
    pub trustfactor: f64,

    trainer_sk: Vec<u8>,

    pub archival_node: bool,
    pub autoupdate: bool,
    pub computor_type: ComputorType,
    pub snapshot_height: u64,
}

impl AmaConfig {
    pub fn version_3b(&self) -> [u8; 3] {
        let trimmed = self.version.trim_start_matches('v');

        // split into parts
        let parts: Vec<&str> = trimmed.split('.').collect();
        assert!(parts.len() == 3, "version must be in vX.Y.Z format");

        // parse into numbers
        let v1: u8 = parts[0].parse().unwrap();
        let v2: u8 = parts[1].parse().unwrap();
        let v3: u8 = parts[2].parse().unwrap();

        [v1, v2, v3]
    }

    pub fn trainer_sk(&self) -> [u8; 64] {
        let sk_bytes = &self.trainer_sk;

        if sk_bytes.len() == 32 {
            if let Ok(_sk) = SecretKey::from_bytes(sk_bytes) {
                // Return 64 bytes: duplicate the 32-byte key
                let mut full_sk = [0u8; 64];
                full_sk[..32].copy_from_slice(sk_bytes);
                full_sk[32..].copy_from_slice(sk_bytes); // optional duplication
                return full_sk;
            }
        }

        // Fallback: generate 64 random bytes
        let mut sk = [0u8; 64];
        OsRng
            .try_fill_bytes(&mut sk)
            .expect("Failed to generate random bytes");
        println!("Generated random 64 bytes: {:?}", sk);
        sk
    }

    pub fn trainer_pk(&self) -> Vec<u8> {
        BlsRs::get_public_key(&self.trainer_sk).unwrap()
    }

    pub fn trainer_pop(&self) -> Vec<u8> {
        BlsRs::sign(&self.trainer_sk, &self.trainer_pk(), BLS12AggSig::DST_POP).unwrap()
    }

    //     if !Util.verify_time_sync() do
    //     IO.puts "🔴 🕒 time not synced OR systemd-ntp client not found; DYOR 🔴"
    // end
}

// pub AMACONFIG : AmaConfig= CONFIG.
pub static AMACONFIG: Lazy<AmaConfig> = Lazy::new(|| CONFIG.ama.clone());
