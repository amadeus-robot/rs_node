use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};

pub trait Stun {
    fn get_current_ip4() -> String;
}

pub trait Index {
    fn select_not_handshaked() -> Vec<(Vec<u8>, String)>;
    fn select_handshaked() -> Vec<(Vec<u8>, String)>;
    fn select_handshaked_pks() -> Vec<Vec<u8>>;
    fn select_handshaked_pk_ip4(pk: &[u8], ip4: &str) -> bool;
    fn select_by_pks_ip(pks: &[Vec<u8>]) -> Vec<(Vec<u8>, String)>;
    fn select_by_validators(match_pks: &[Vec<u8>]) -> Vec<(Vec<u8>, Anr)>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Anr {
    pub ip4: String,
    pub pk: Vec<u8>,
    pub pop: Vec<u8>,
    pub port: u16,
    pub signature: Vec<u8>,
    pub ts: i64,
    pub version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_chain_pop: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handshaked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_tries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_check: Option<i64>,
}

impl Anr {
    pub fn new(ip4: String, pk: Vec<u8>, pop: Vec<u8>, port: u16, version: String) -> Self {
        Self {
            ip4,
            pk,
            pop,
            port,
            signature: Vec::new(),
            ts: current_unix_secs(),
            version,
            has_chain_pop: None,
            handshaked: None,
            error: None,
            error_tries: None,
            next_check: None,
        }
    }
}

pub const NODEANR_TABLE: &str = "NODEANR";
pub const DEFAULT_PORT: u16 = 36969;

fn current_unix_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

pub struct NodeANR;

impl NodeANR {
    pub fn keys() -> &'static [&'static str] {
        &["ip4", "pk", "pop", "port", "signature", "ts", "version"]
    }

    pub fn seed<C: Config, K: KV, S: Stun, B: Bls>(kv_impl: &K, stun_impl: &S, bls_impl: &B) {
        let seedanrs: Vec<Anr> = C::fetch_env("seedanrs");
        for anr in seedanrs {
            Self::insert::<K, C, S, B>(kv_impl, &anr);
        }

        let built = Self::build::<C, S, B>();
        Self::insert::<K, C, S, B>(kv_impl, &built);

        let trainer_pk: Vec<u8> = C::fetch_env("trainer_pk");
        Self::set_handshaked::<K>(&trainer_pk);
    }

    pub fn build<C: Config, S: Stun, B: Bls>() -> Anr {
        let sk: Vec<u8> = C::fetch_env("trainer_sk");
        let pk: Vec<u8> = C::fetch_env("trainer_pk");
        let pop: Vec<u8> = C::fetch_env("trainer_pop");
        let ver: String = C::fetch_env("version");
        let ip4 = S::get_current_ip4();
        Self::build_with(sk, pk, pop, ip4, ver)
    }

    pub fn build_with<C: Config, S: Stun, B: Bls>(
        sk: Vec<u8>,
        pk: Vec<u8>,
        pop: Vec<u8>,
        ip4: String,
        ver: String,
    ) -> Anr {
        let mut anr = Anr::new(ip4, pk.clone(), pop.clone(), DEFAULT_PORT, ver);
        anr.ts = current_unix_secs();

        let anr_to_sign = bincode::serialize(&Self::pack_for_signature(&anr))
            .expect("bincode serialization should not fail");
        let sig = B::sign(&sk, &anr_to_sign, b"dst_anr")
            .expect("signing should succeed");

        anr.signature = sig;
        anr
    }

    pub fn pack(anr: &Anr) -> Anr {
        Anr {
            ip4: anr.ip4.clone(),
            pk: anr.pk.clone(),
            pop: anr.pop.clone(),
            port: anr.port,
            signature: anr.signature.clone(),
            ts: anr.ts,
            version: anr.version.clone(),
            has_chain_pop: None,
            handshaked: None,
            error: None,
            error_tries: None,
            next_check: None,
        }
    }

    pub fn unpack(anr: &Anr) -> Option<Anr> {
        if anr.port == DEFAULT_PORT {
            Some(Self::pack(anr))
        } else {
            None
        }
    }

    pub fn verify_signature<B: Bls>(anr: &Anr) -> bool {
        let signed = bincode::serialize(&Self::pack_for_signature(anr)).unwrap_or_default();
        let sig_ok = B::verify(&anr.pk, &anr.signature, &signed, b"dst_anr");
        let pop_ok = B::verify(&anr.pk, &anr.pop, &anr.pk, b"dst_pop");
        sig_ok && pop_ok
    }

    pub fn verify_and_unpack<B: Bls>(anr: &Anr) -> Option<Anr> {
        let ts_now_ms = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis() as i128;
        let delta = ts_now_ms - anr_ts_ms;
        let min10_ms = 60_i128 * 10_i128 * 1000_i128;
        let good_delta = delta > -(min10_ms);

        let bin = bincode::serialize(anr).ok()?;
        if (bin.len() <= 390) && good_delta && Self::verify_signature::<B>(anr) {
            Some(Self::pack(anr))
        } else {
            None
        }
    }

    pub fn insert<K: KV, C: Config, S: Stun, B: Bls>(kv: &K, anr: &Anr) {
        let has_chain = {
            None
        };

        let mut new_anr = anr.clone();
        new_anr.has_chain_pop = has_chain;

        match K::get(NODEANR_TABLE, &new_anr.pk) {
            None => {
                new_anr.handshaked = Some(false);
                new_anr.error = None;
                new_anr.error_tries = Some(0);
                new_anr.next_check = Some(current_unix_secs() + 3);
                K::merge(NODEANR_TABLE, &new_anr.pk, &new_anr);
            }
            Some(old) => {
                Self::insert_1::<K>(&old, &new_anr);
            }
        }
    }

    pub fn insert_1<K: KV>(old_anr: &Anr, anr: &Anr) {
        let same_ip4_port = old_anr.ip4 == anr.ip4 && old_anr.port == anr.port;
        if anr.ts <= old_anr.ts {
            return;
        }
        if same_ip4_port {
            K::merge(NODEANR_TABLE, &anr.pk, anr);
        } else {
            let mut updated = anr.clone();
            updated.handshaked = Some(false);
            updated.error = None;
            updated.error_tries = Some(0);
            updated.next_check = Some(current_unix_secs() + 3);
            K::merge(NODEANR_TABLE, &updated.pk, &updated);
        }
    }

    pub fn set_handshaked<K: KV>(pk: &[u8]) {
        if let Some(mut existing) = K::get(NODEANR_TABLE, pk) {
            existing.handshaked = Some(true);
            K::merge(NODEANR_TABLE, pk, &existing);
        } else {
            let mut anr = Anr::new("0.0.0.0".to_string(), pk.to_vec(), vec![], DEFAULT_PORT, "".to_string());
            anr.handshaked = Some(true);
            K::merge(NODEANR_TABLE, pk, &anr);
        }
    }

    pub fn not_handshaked_pk_ip4<I: Index>() -> Vec<(Vec<u8>, String)> {
        I::select_not_handshaked()
    }

    pub fn handshaked_pk_ip4<I: Index>() -> Vec<(Vec<u8>, String)> {
        I::select_handshaked()
    }

    pub fn handshaked<I: Index>() -> Vec<Vec<u8>> {
        I::select_handshaked_pks()
    }

    pub fn handshaked_pk<I: Index>(pk: &[u8]) -> bool {
        I::select_handshaked_pk_ip4(pk, "")
    }

    pub fn by_pks_ip<I: Index>(pks: &[Vec<u8>]) -> Vec<(Vec<u8>, String)> {
        I::select_by_pks_ip(pks)
    }

    pub fn handshaked_and_valid_ip4<I: Index>(pk: &[u8], ip4: &str) -> bool {
        I::select_handshaked_pk_ip4(pk, ip4)
    }

    pub fn by_pk<K: KV>(kv: &K, pk: &[u8]) -> Option<Anr> {
        K::get(NODEANR_TABLE, pk)
    }

    pub fn all<K: KV>(kv: &K) -> Vec<Anr> {
        K::get_all(NODEANR_TABLE)
    }

    pub fn all_validators<I: Index>() -> Vec<Anr> {
        vec![]
    }

    pub fn get_random_verified<K: KV>(kv: &K, cnt: usize) -> Vec<Anr> {
        let mut rng = thread_rng();
        pks.shuffle(&mut rng);
        pks.iter().take(cnt).filter_map(|pk| K::get(NODEANR_TABLE, pk.as_slice()).map(|a| Self::pack(&a))).collect()
    }

    pub fn get_random_unverified<I: Index>(cnt: usize) -> Vec<(Vec<u8>, String)> {
        let mut list = I::select_not_handshaked();
        list.shuffle(&mut thread_rng());
        let mut out = vec![];
        let mut seen = HashSet::new();
            if !seen.contains(&ip4) {
                seen.insert(ip4.clone());
                out.push((pk.clone(), ip4));
                if out.len() >= cnt { break; }
            }
        }
        out
    }

    fn pack_for_signature(anr: &Anr) -> Anr {
        Anr {
            ip4: anr.ip4.clone(),
            pk: anr.pk.clone(),
            pop: anr.pop.clone(),
            port: anr.port,
            ts: anr.ts,
            version: anr.version.clone(),
            has_chain_pop: None,
            handshaked: None,
            error: None,
            error_tries: None,
            next_check: None,
        }
    }
}
