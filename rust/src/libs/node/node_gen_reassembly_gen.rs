// src/node_gen_reassembly_gen.rs
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task;
use tokio::time::Instant;
use tokio::time::sleep;

use anyhow::Result;
use blake3; // make sure to add to Cargo.toml if you use it
use bytes::Bytes;

/// ------------------------
/// Placeholders / Traits
/// ------------------------
/// Implement these with your project's real code (or swap for crates).
pub trait ReedSolomon {
    /// create resource; in Elixir you used (data_shards, parity_shards, shard_size)
    fn create_resource(data_shards: usize, parity_shards: usize, _shard_size: usize) -> Self
    where
        Self: Sized;
    fn decode_shards(
        &self,
        shards: Vec<(usize, Vec<u8>)>,
        shard_total: usize,
        original_size: usize,
    ) -> Result<Vec<u8>>;
}

pub trait Bls {
    /// verify signature: returns true if valid
    fn verify(pk: &[u8], signature: &[u8], message: &[u8], dst: &[u8]) -> bool;
}

pub trait NodeProto {
    /// decompress + other deflate logic; returns decompressed bytes
    fn deflate_decompress(input: &[u8]) -> Result<Vec<u8>>;
}

pub trait NodeState {
    /// handle an op; identical in concept to NodeState.handle(op, %{peer...}, msg)
    fn handle(op: &str, peer: Peer, msg: serde_json::Value);
}

/// -------------
/// Types
/// -------------
#[derive(Clone, Debug)]
pub struct Peer {
    pub ip: String,
    pub version: [u8; 3],
    pub signer: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct AddShardPayload {
    pub pk: Vec<u8>,
    pub ts_nano: i128,
    pub shard_total: usize,
    pub ip: String,
    pub version_3byte: [u8; 3],
    pub shared_secret: Vec<u8>,
    pub signature: Option<Vec<u8>>,
    pub shard_index: usize,
    pub original_size: usize,
    pub shard: Vec<u8>,
}

/// Key used in the reorg map (equivalent to {pk, ts_nano, shard_total})
#[derive(Clone, Debug, Eq)]
pub struct ReorgKey {
    pub pk: Vec<u8>,
    pub ts_nano: i128,
    pub shard_total: usize,
}

impl PartialEq for ReorgKey {
    fn eq(&self, other: &Self) -> bool {
        self.pk == other.pk
            && self.ts_nano == other.ts_nano
            && self.shard_total == other.shard_total
    }
}

impl Hash for ReorgKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pk.hash(state);
        self.ts_nano.hash(state);
        self.shard_total.hash(state);
    }
}

/// The per-key stored shards map: shard_index -> shard_bytes
type ShardMap = HashMap<usize, Vec<u8>>;

/// The actor/state
pub struct NodeGenReassemblyGen<R: ReedSolomon, B: Bls, P: NodeProto, S: NodeState> {
    name: String,
    reorg: HashMap<ReorgKey, ShardMapOrSpent>,
    tx: Sender<Message>,
    _r: std::marker::PhantomData<R>,
    _b: std::marker::PhantomData<B>,
    _p: std::marker::PhantomData<P>,
    _s: std::marker::PhantomData<S>,
}

/// Stores either the map of shards or a sentinel `Spent` (equivalent to :spent)
#[derive(Clone, Debug)]
pub enum ShardMapOrSpent {
    Map(ShardMap),
    Spent,
}

/// Messages sent to the actor
#[derive(Clone, Debug)]
pub enum Message {
    Tick,
    AddShard(AddShardPayload),
}

impl<R, B, P, S> NodeGenReassemblyGen<R, B, P, S>
where
    R: ReedSolomon + Send + 'static,
    B: Bls + Send + 'static,
    P: NodeProto + Send + 'static,
    S: NodeState + Send + 'static,
{
    /// start_link(name) -> returns Sender<Message> to send messages to actor.
    pub async fn start_link(name: Option<&str>) -> Sender<Message> {
        let actor_name = name.unwrap_or("NodeGenReassemblyGen").to_string();
        let (tx, rx) = mpsc::channel(256);
        let mut actor = Self {
            name: actor_name,
            reorg: HashMap::new(),
            tx: tx.clone(),
            _r: Default::default(),
            _b: Default::default(),
            _p: Default::default(),
            _s: Default::default(),
        };

        task::spawn(async move {
            actor.init(rx).await;
        });

        let tx_clone = tx.clone();
        task::spawn(async move {
            sleep(Duration::from_millis(8000)).await;
            let _ = tx_clone.send(Message::Tick).await;
        });

        tx
    }

    /// init loop: processes incoming messages
    async fn init(&mut self, mut rx: Receiver<Message>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::Tick => {
                    self.handle_tick().await;
                    let tx_clone = self.tx.clone();
                    task::spawn(async move {
                        sleep(Duration::from_millis(8000)).await;
                        let _ = tx_clone.send(Message::Tick).await;
                    });
                }
                Message::AddShard(payload) => {
                    self.handle_add_shard(payload).await;
                }
            }
        }
    }

    /// clear_stale: remove any reorg entries older than threshold (now - 8_000_000_000 ns)
    fn clear_stale(&mut self) {
        let now_nano = now_nanos();
        let threshold = now_nano - 8_000_000_000i128;
        self.reorg.retain(|key, _val| key.ts_nano > threshold);
    }

    async fn handle_tick(&mut self) {
        self.clear_stale();
    }

    /// The heavy function: mirrors handle_info({:add_shard, ...}, state)
    async fn handle_add_shard(&mut self, p: AddShardPayload) {
        let key = ReorgKey {
            pk: p.pk.clone(),
            ts_nano: p.ts_nano,
            shard_total: p.shard_total,
        };

        match self.reorg.get_mut(&key) {
            None => {
                let mut map = HashMap::new();
                map.insert(p.shard_index, p.shard);
                self.reorg.insert(key, ShardMapOrSpent::Map(map));
                return;
            }
            Some(ShardMapOrSpent::Spent) => {
                return;
            }
            Some(ShardMapOrSpent::Map(map)) => {
                if map.len() < (p.shard_total / 2usize).saturating_sub(1) {
                    map.insert(p.shard_index, p.shard);
                    return;
                } else {
                    let mut shards_vec: Vec<(usize, Vec<u8>)> =
                        map.iter().map(|(k, v)| (*k, v.clone())).collect();
                    shards_vec.push((p.shard_index, p.shard));
                    self.reorg.insert(key.clone(), ShardMapOrSpent::Spent);

                    task::spawn_blocking(move || {
                        let res = std::panic::catch_unwind(|| {
                            let r = R::create_resource(p.shard_total / 2, p.shard_total / 2, 1024);
                            match r.decode_shards(shards_vec, p.shard_total, p.original_size) {
                                Ok(payload) => {
                                    let _ = proc_msg::<B, P, S>(
                                        p.pk,
                                        p.shared_secret,
                                        p.signature,
                                        p.ts_nano,
                                        p.ip,
                                        p.version_3byte,
                                        payload,
                                    );
                                }
                                Err(e) => {
                                    eprintln!("msg_reassemble_failed decode error: {:?}", e);
                                }
                            }
                        });

                        if let Err(e) = res {
                            eprintln!("msg_reassemble_failed panic: {:?}", e);
                        }
                    });
                }
            }
        }
    }
}

/// Equivalent of Elixir's proc_msg (but returns Result<()>)
/// Note: this spawns a tokio task to call NodeState::handle (mirrors :erlang.spawn)
fn proc_msg<B, P, S>(
    pk: Vec<u8>,
    shared_secret: Vec<u8>,
    signature: Option<Vec<u8>>,
    ts_nano: i128,
    ip: String,
    version_3byte: [u8; 3],
    payload: Vec<u8>,
) -> Result<()>
where
    B: Bls + Send + 'static,
    P: NodeProto + Send + 'static,
    S: NodeState + Send + 'static,
{
    let res = std::panic::catch_unwind(|| {
        if let Some(sig) = signature {
            let mut hasher = blake3::Hasher::new();
            hasher.update(&pk);
            hasher.update(&payload);
            let hash = hasher.finalize();
            let hash_bytes = hash.as_bytes();

            let ok = B::verify(&pk, &sig, hash_bytes, b"dst_node");
            if ok {
                match P::deflate_decompress(&payload) {
                    Ok(decompressed) => {
                        let maybe_msg: serde_json::Value = serde_json::from_slice(&decompressed)
                            .unwrap_or(serde_json::Value::Null);

                        let peer = Peer {
                            ip,
                            version: version_3byte,
                            signer: pk.clone(),
                        };

                        task::spawn(async move {
                            S::handle(
                                maybe_msg
                                    .get("op")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default(),
                                peer,
                                maybe_msg,
                            );
                        });
                    }
                    Err(e) => {
                        eprintln!("proc_msg decompress failed: {:?}", e);
                    }
                }
            } else {
            }
        } else {
            if payload.len() < 28 {
                return;
            }
            let iv = &payload[0..12];
            let tag = &payload[12..28];
            let ciphertext = &payload[28..];

            let mut ts_bytes = ts_nano.to_be_bytes().to_vec();
            let mut key_material = Vec::with_capacity(shared_secret.len() + ts_bytes.len());
            key_material.extend_from_slice(&shared_secret);
            key_material.append(&mut ts_bytes);
            key_material.extend_from_slice(iv);

            let key_hash = blake3::hash(&key_material); // you can use SHA256 if you prefer
            let key = key_hash.as_bytes(); // 32 bytes

            //
            let plaintext = {
                panic!("AES-GCM decryption not implemented: replace with real decrypt call");
            };

            match P::deflate_decompress(&plaintext) {
                Ok(decompressed) => {
                    let maybe_msg: serde_json::Value =
                        serde_json::from_slice(&decompressed).unwrap_or(serde_json::Value::Null);
                    let peer = Peer {
                        ip,
                        version: version_3byte,
                        signer: pk.clone(),
                    };
                    task::spawn(async move {
                        S::handle(
                            maybe_msg
                                .get("op")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default(),
                            peer,
                            maybe_msg,
                        );
                    });
                }
                Err(e) => {
                    eprintln!("proc_msg decompress failed after decrypt: {:?}", e);
                }
            }
        }
    });

    match res {
        Ok(_) => Ok(()),
        Err(_) => Ok(()),
    }
}

/// Helper to get current time in nanoseconds (like :os.system_time(:nanosecond))
fn now_nanos() -> i128 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    (dur.as_secs() as i128) * 1_000_000_000i128 + (dur.subsec_nanos() as i128)
}
