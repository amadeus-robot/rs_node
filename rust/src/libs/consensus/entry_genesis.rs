use bls12_381_plus::{G1Projective, Scalar};
use blake3::Hasher;
use std::sync::OnceLock;

use crate::*;

pub struct EntryGenesis {
    pub signer: Vec<u8>,
    pub pop: Vec<u8>,
    pub attestation: Attestation,
    pub genesis_entry: Entry,
}

#[derive(Clone, Debug)]
pub struct Attestation {
    pub signature: Vec<u8>,
    pub mutations_hash: Vec<u8>,
    pub signer: Vec<u8>,
    pub entry_hash: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct EntryHeader {
    pub slot: i64,
    pub height: i64,
    pub signer: Vec<u8>,
    pub vr: Vec<u8>,
    pub prev_hash: Vec<u8>,
    pub dr: Vec<u8>,
    pub prev_slot: i64,
    pub txs_hash: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub header_unpacked: EntryHeader,
    pub txs: Vec<Vec<u8>>,
    pub hash: Vec<u8>,
    pub signature: Vec<u8>,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub struct Config {
    pub trainer_pk: Vec<u8>,
    pub trainer_sk: Vec<u8>,
}

impl EntryGenesis {
    pub fn new() -> Self {
        // Replace these with the actual byte arrays from Elixir module
        let signer = vec![140, 27, 75, 245, 48, 112, 140, 244, 78, 114, 11, 45, 8, 201, 199,
            184, 71, 69, 96, 112, 52, 204, 31, 56, 143, 115, 222, 87, 7, 185, 3, 168,
            252, 90, 91, 114, 16, 244, 47, 228, 198, 82, 12, 130, 10, 126, 118, 193];

        let pop = vec![175, 176, 86, 129, 118, 228, 182, 86, 225, 187, 236, 131, 170, 81, 121, 174,
            164, 44, 71, 123, 136, 151, 170, 187, 43, 43, 211, 181, 163, 103, 93, 122,
            11, 207, 92, 1, 190, 71, 46, 129, 210, 134, 62, 169, 152, 161, 189, 58, 18,
            246, 6, 151, 128, 196, 116, 93, 20, 204, 153, 217, 81, 205, 1, 133, 65, 204,
            177, 138, 74, 8, 104, 109, 214, 59, 245, 51, 47, 218, 15, 207, 190, 73, 40,
            128, 108, 147, 250, 88, 241, 61, 129, 47, 189, 173, 118, 76];

        let attestation = Attestation {
            signature: vec![], // fill with actual bytes
            mutations_hash: vec![],
            signer: signer.clone(),
            entry_hash: vec![],
        };

        let genesis_entry = Entry {
            header_unpacked: EntryHeader {
                slot: 0,
                height: 0,
                signer: signer.clone(),
                vr: vec![],
                prev_hash: vec![],
                dr: vec![],
                prev_slot: -1,
                txs_hash: vec![],
            },
            txs: vec![],
            hash: vec![],
            signature: vec![],
        };

        EntryGenesis {
            signer,
            pop,
            attestation,
            genesis_entry,
        }
    }

    pub fn signer(&self) -> &[u8] {
        &self.signer
    }

    pub fn pop(&self) -> &[u8] {
        &self.pop
    }

    pub fn attestation(&self) -> &Attestation {
        &self.attestation
    }

    pub fn get(&self) -> &Entry {
        &self.genesis_entry
    }

    pub fn generate(&self) {
        let config = CONFIG.get().expect("Config not initialized");
        let pk = &config.trainer_pk;
        let sk = &config.trainer_sk;

        let entropy_seed = b"January 27, 2025\n\nTech stocks tank as a Chinese competitor threatens to upend the AI frenzy; Nvidia sinks nearly 17%";
        let dr = Blake3::hash(entropy_seed);

        // VRF: simplified placeholder, use actual BLS signing
        let vr_input = [dr.as_bytes(), dr.as_bytes(), dr.as_bytes()].concat();
        let vr = bls_sign(sk, &vr_input);

        let entry_header = EntryHeader {
            slot: 0,
            height: 0,
            prev_slot: -1,
            prev_hash: vec![],
            dr: dr.as_bytes().to_vec(),
            vr,
            signer: pk.clone(),
            txs_hash: vec![],
        };

        let entry = Entry {
            header_unpacked: entry_header,
            txs: vec![],
            hash: vec![],
            signature: vec![],
        };

        // TODO: call exit and mutations, produce attestation, PoP, etc.
        println!("Generated entry: {:?}", entry);
    }
}

// Placeholder function for BLS signing
fn bls_sign(_sk: &Vec<u8>, msg: &[u8]) -> Vec<u8> {
    // Replace with real BLS signing
    msg.to_vec()
}

// Wrapper for Blake3
mod Blake3 {
    pub fn hash(input: &[u8]) -> blake3::Hash {
        blake3::hash(input)
    }
}
