// node_state.rs
// Prototype Rust port of Elixir NodeState
// Requires tokio (for async spawn), serde (for simple packing/unpacking), and parking_lot (for locks).
// This is a behavioral port — domain modules are left as stubs to be implemented.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use tokio::task;

#[derive(Clone, Debug)]
pub struct ANR {
    pub ip4: String,
    pub pk: Vec<u8>,
    pub version: String,
}

pub trait NodeANRApi {
    fn verify_and_unpack(packed: &[u8]) -> Option<ANR>;
    fn insert(anr: &ANR);
    fn set_handshaked(pk: &[u8]);
    fn handshaked_and_valid_ip4(pk: &[u8], ip: &str) -> bool;
    fn get_random_verified(n: usize) -> Vec<ANR>;
    fn get_shared_secret(pk: &[u8], sk: &[u8]) -> Vec<u8>;
}

pub struct NodeANR;
impl NodeANRApi for NodeANR {
    fn verify_and_unpack(_packed: &[u8]) -> Option<ANR> { None }
    fn insert(_anr: &ANR) {}
    fn set_handshaked(_pk: &[u8]) {}
    fn handshaked_and_valid_ip4(_pk: &[u8], _ip: &str) -> bool { false }
    fn get_random_verified(_n: usize) -> Vec<ANR> { vec![] }
    fn get_shared_secret(_pk: &[u8], _sk: &[u8]) -> Vec<u8> { vec![] }
}

#[derive(Clone)]
pub struct SocketMessage {
    pub to_ips: Vec<String>,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Peer {
    pub ip: String,
    pub signer: Vec<u8>,
    pub version: String,
    pub shared_secret: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct IState {
    pub peer: Peer,
    pub ns: HashMap<String, Vec<u8>>, // placeholder for ns state
}

// Define incoming message types (term)
#[derive(Clone)]
pub enum NodeMsg {
    NewPhoneWhoDis { anr: Vec<u8>, challenge: i64 },
    NewPhoneWhoDisNs { anr: Vec<u8> },
    What { anr: Vec<u8>, signature: Vec<u8>, challenge: i64 },
    WhatNs { anr: Vec<u8>, pk: Vec<u8> },
    Ping { temporal: Vec<u8>, rooted: Vec<u8>, ts_m: i64 },
    PingNs { temporal: Vec<u8>, rooted: Vec<u8>, ts_m: i64, has_permission_slip: bool },
    Pong { ts_m: i64 },
    PongNs { seen_time: i64, ts_m: i64 },
    TxPool { txs_packed: Vec<Vec<u8>> },
    PeersV2 { anrs: Vec<Vec<u8>> },
    PeersV2Ns { anrs: Vec<ANR> },
    Sol { sol: Vec<u8> },
    Entry { entry_packed: Vec<u8>, consensus_packed: Option<Vec<u8>>, attestation_packed: Option<Vec<u8>>, ts_m: Option<i64> },
    AttestationBulk { attestations_packed: Vec<Vec<u8>> },
    ConsensusBulk { consensuses_packed: Vec<Vec<u8>> },
    CatchupEntry { heights: Vec<u64> },
    CatchupTri { heights: Vec<u64> },
    CatchupBi { heights: Vec<u64> },
    CatchupAttestation { hashes: Vec<Vec<u8>> },
    SpecialBusiness { business_op: String, business_args: HashMap<String, Vec<u8>> },
    SpecialBusinessReply { business_op: String, business_args: HashMap<String, Vec<u8>> },
    SolicitEntry { hash: Vec<u8> },
    SolicitEntry2,
    Unknown,
}

// ---------- NodeState implementation ----------

pub struct NodeState {
    challenges: Arc<RwLock<HashMap<Vec<u8>, i64>>>,
    peers: Arc<RwLock<HashMap<String, Peer>>>,
    socket_sender: UnboundedSender<SocketMessage>,
    trainer_pk: Vec<u8>,
    trainer_sk: Vec<u8>,
}

impl NodeState {
    pub fn new(trainer_sk: Vec<u8>, trainer_pk: Vec<u8>) -> Self {
        let (socket_sender, _socket_recv) = unbounded_channel::<SocketMessage>();
        NodeState {
            challenges: Arc::new(RwLock::new(HashMap::new())),
            peers: Arc::new(RwLock::new(HashMap::new())),
            socket_sender,
            trainer_pk,
            trainer_sk,
        }
    }

    fn now_secs() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
    }
    fn now_millis() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
    }

    pub async fn handle(&self, tag: NodeMsg, istate: &mut IState) {
        match tag {
            NodeMsg::NewPhoneWhoDis { anr, challenge } => {
                self.handle_new_phone_who_dis(istate, anr, challenge).await;
            }
            NodeMsg::NewPhoneWhoDisNs { anr } => {
                if let Some(anr_struct) = NodeANR::verify_and_unpack(&anr) {
                    NodeANR::insert(&anr_struct);
                }
            }
            NodeMsg::What { anr, signature, challenge } => {
                self.handle_what(istate, anr, signature, challenge).await;
            }
            NodeMsg::WhatNs { anr, pk: _ } => {
                if let Some(anr_struct) = NodeANR::verify_and_unpack(&anr) {
                    NodeANR::insert(&anr_struct);
                    NodeANR::set_handshaked(&anr_struct.pk);
                }
            }
            NodeMsg::Ping { temporal, rooted, ts_m } => {
                self.handle_ping(istate, temporal, rooted, ts_m).await;
            }
            NodeMsg::PingNs { temporal, rooted, ts_m, has_permission_slip } => {
                self.handle_ping_ns(istate, temporal, rooted, ts_m, has_permission_slip).await;
            }
            NodeMsg::Pong { ts_m } => {
                self.handle_pong(istate, ts_m).await;
            }
            NodeMsg::PongNs { seen_time, ts_m } => {
                self.handle_pong_ns(istate, seen_time, ts_m).await;
            }
            NodeMsg::TxPool { txs_packed } => {
                let good: Vec<Vec<u8>> = txs_packed.into_iter()
                    .filter(|tx| {
                        true
                    }).collect();
                TXPool::insert(good);
            }
            NodeMsg::PeersV2 { anrs } => {
                let verified: Vec<ANR> = anrs.into_iter()
                    .filter_map(|p| NodeANR::verify_and_unpack(&p))
                    .collect();
                for anr in verified {
                    NodeANR::insert(&anr);
                }
            }
            NodeMsg::Sol { sol } => {
                self.handle_sol(istate, sol).await;
            }
            NodeMsg::Entry { entry_packed, consensus_packed, attestation_packed, ts_m: _ } => {
                self.handle_entry(istate, entry_packed, consensus_packed, attestation_packed).await;
            }
            _ => {
            }
        }
    }

    async fn handle_new_phone_who_dis(&self, istate: &IState, anr_packed: Vec<u8>, challenge: i64) {
        if let Some(anr) = NodeANR::verify_and_unpack(&anr_packed) {
            if istate.peer.ip == anr.ip4 && challenge.is_positive() {
                let sk = &self.trainer_sk;
                let pk = &self.trainer_pk;
                let mut msg = Vec::new();
                msg.extend(pk.iter());
                msg.extend(challenge.to_string().as_bytes());
                let sig = BlsEx::sign(sk, &msg, b"dst_anr_challenge"); // dst placeholder
                let socket_msg = SocketMessage { to_ips: vec![istate.peer.ip.clone()], payload: compress(&what_msg(challenge, &sig)) };
                let _ = self.socket_sender.send(socket_msg);
                let mut payload = HashMap::new();
                payload.insert("anr".to_string(), anr_packed);
                NodeGen::handle_sync("new_phone_who_dis_ns", istate, payload);
            }
        }
    }

    async fn handle_what(&self, istate: &IState, anr_packed: Vec<u8>, signature: Vec<u8>, challenge: i64) {
        if let Some(anr) = NodeANR::verify_and_unpack(&anr_packed) {
            let ts = Self::now_secs();
            let delta = (ts - challenge).abs();
            if istate.peer.ip == anr.ip4 && delta <= 6 {
                let mut msg = Vec::new();
                msg.extend(anr.pk.iter());
                msg.extend(challenge.to_string().as_bytes());
                let sig_ok = BlsEx::verify(&anr.pk, &signature, &msg, b"dst_anr_challenge");
                if sig_ok {
                    let mut payload = HashMap::new();
                    payload.insert("pk".to_string(), anr.pk.clone());
                    NodeGen::handle_sync("what?_ns", istate, payload);
                }
            }
        }
    }

    async fn handle_ping(&self, istate: &IState, temporal_packed: Vec<u8>, rooted_packed: Vec<u8>, ts_m: i64) {
        let temporal_res = Entry::validate_signature(&[], &[], &[], None);
        let rooted_res = Entry::validate_signature(&[], &[], &[], None);

        match (temporal_res, rooted_res) {
            (Ok(hash_t), Ok(hash_r)) => {
                let has_permission_slip = NodeANR::handshaked_and_valid_ip4(&istate.peer.signer, &istate.peer.ip);
                let peer_ip = istate.peer.ip.clone();
                let ss = self.socket_sender.clone();
                let has_permission_slip_clone = has_permission_slip;
                task::spawn(async move {
                    if has_permission_slip_clone {
                        let anrs = NodeANR::get_random_verified(3);
                        if !anrs.is_empty() {
                            let payload = vec![]; // placeholder
                            let _ = ss.send(SocketMessage { to_ips: vec![peer_ip], payload: compress(&peers_v2_msg(&anrs)) });
                        }
                    }
                });

                let mut payload = HashMap::new();
                payload.insert("temporal_hash".to_string(), hash_t);
                payload.insert("rooted_hash".to_string(), hash_r);
                payload.insert("ts_m".to_string(), ts_m.to_string().into_bytes());
                payload.insert("hasPermissionSlip".to_string(), if has_permission_slip { vec![1] } else { vec![0] });
                NodeGen::handle_sync("ping_ns", istate, payload);
            }
            Err(_e) => {
            }
        }
    }

    async fn handle_ping_ns(&self, istate: &IState, temporal_packed: Vec<u8>, rooted_packed: Vec<u8>, ts_m: i64, has_permission_slip: bool) {
        let peer_ip = &istate.peer.ip;
        if has_permission_slip /* or istate.peer.signer in Consensus.trainers_for_height(...) */ {
            let mut peers_lock = self.peers.write();
            let peer = peers_lock.entry(peer_ip.clone()).or_insert_with(|| istate.peer.clone());
            peer.ip = peer_ip.clone();
            peer.version = istate.peer.version.clone();
            if peer.shared_secret.is_none() {
                let shared_key = BlsEx::get_shared_secret(&istate.peer.signer, &self.trainer_sk);
                peer.shared_secret = Some(shared_key);
            }
            let ss = self.socket_sender.clone();
            let peer_ip_clone = peer_ip.clone();
            task::spawn(async move {
                let _ = ss.send(SocketMessage { to_ips: vec![peer_ip_clone], payload: compress(&pong_msg(ts_m)) });
            });
        }
    }

    async fn handle_pong(&self, istate: &IState, ts_m: i64) {
        let seen_time = Self::now_millis();
        let mut payload = HashMap::new();
        payload.insert("seen_time".to_string(), seen_time.to_string().into_bytes());
        payload.insert("ts_m".to_string(), ts_m.to_string().into_bytes());
        NodeGen::handle_sync("pong_ns", istate, payload);
    }

    async fn handle_pong_ns(&self, _istate: &IState, seen_time: i64, ts_m: i64) {
        let peer_ip = &_istate.peer.ip; // in practice you'd pass istate
        let latency = seen_time - ts_m;
        let mut peers_lock = self.peers.write();
        if let Some(peer) = peers_lock.get_mut(peer_ip) {
        } else {
        }
    }

    async fn handle_sol(&self, istate: &IState, sol_packed: Vec<u8>) {
    }

    async fn handle_entry(&self, _istate: &IState, entry_packed: Vec<u8>, consensus_packed: Option<Vec<u8>>, attestation_packed: Option<Vec<u8>>) {
    }
}

fn compress(payload: &[u8]) -> Vec<u8> {
    payload.to_vec()
}

fn what_msg(challenge: i64, signature: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend(challenge.to_string().as_bytes());
    v.extend(signature);
    v
}

fn peers_v2_msg(anrs: &[ANR]) -> Vec<u8> {
    vec![]
}

fn pong_msg(ts_m: i64) -> Vec<u8> {
    ts_m.to_string().into_bytes()
}