use wasmer::{
    AsStoreMut, Engine, Function, FunctionEnv, FunctionEnvMut, FunctionType, Global, Instance,
    Memory, MemoryType, MemoryView, Module, Pages, RuntimeError, Store, Type, Value, imports,
    sys::{EngineBuilder, Features},
    wasmparser::Operator,
};
use wasmer_compiler_singlepass::Singlepass;

use std::sync::{Arc, Mutex, OnceLock};
use wasmer_middlewares::{
    Metering,
    metering::{MeteringPoints, get_remaining_points, set_remaining_points},
};

use std::collections::HashMap;

use sha2::{Digest, Sha256};
static MODULE_CACHE: OnceLock<Mutex<HashMap<[u8; 32], (Arc<Engine>, Arc<Module>)>>> =
    OnceLock::new();

use rand::random;
use std::sync::{LazyLock, mpsc};
static REQ_REGISTRY_STORAGE_KV_GET: LazyLock<Mutex<HashMap<u64, mpsc::Sender<Option<Vec<u8>>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static REQ_REGISTRY_STORAGE_KV_EXISTS: LazyLock<Mutex<HashMap<u64, mpsc::Sender<bool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static REQ_REGISTRY_STORAGE_KV_GET_PREV_NEXT: LazyLock<
    Mutex<HashMap<u64, mpsc::Sender<(Option<Vec<u8>>, Option<Vec<u8>>)>>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

static REQ_REGISTRY_STORAGE: LazyLock<Mutex<HashMap<u64, mpsc::Sender<Vec<u8>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static REQ_REGISTRY_CALL: LazyLock<
    Mutex<HashMap<u64, mpsc::Sender<(Vec<u8>, Vec<Vec<u8>>, u64, Option<Vec<u8>>)>>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

// Host environment for WASM execution
#[derive(Clone)]
struct HostEnv {
    memory: Option<Memory>,
    readonly: bool,
    error: Option<Vec<u8>>,
    return_value: Option<Vec<u8>>,
    logs: Vec<Vec<u8>>,
    current_account: Vec<u8>,
    attached_symbol: Vec<u8>,
    attached_amount: Vec<u8>,
    instance: Option<Arc<Instance>>,
}

pub struct WasmerRs;

impl WasmerRs {
    // Example function: respond to KV get without Rustler
    fn respond_to_storage_kv_get(request_id: u64, response: Option<Vec<u8>>) -> Result<(), String> {
        let mut map = REQ_REGISTRY_STORAGE_KV_GET.lock().unwrap();

        if let Some(tx) = map.remove(&request_id) {
            tx.send(response).map_err(|_| "send failed".to_string())
        } else {
            Err("no request found".to_string())
        }
    }

    fn respond_to_rust_storage_kv_exists(request_id: u64, response: bool) -> Result<(), String> {
        let mut map = REQ_REGISTRY_STORAGE_KV_EXISTS.lock().unwrap();

        if let Some(tx) = map.remove(&request_id) {
            tx.send(response).map_err(|_| "send failed".to_string())
        } else {
            Err("no request found".to_string())
        }
    }

    fn respond_to_rust_storage_kv_get_prev_next(
        request_id: u64,
        response: (Option<Vec<u8>>, Option<Vec<u8>>),
    ) -> Result<(), String> {
        let mut map = REQ_REGISTRY_STORAGE_KV_GET_PREV_NEXT.lock().unwrap();

        if let Some(tx) = map.remove(&request_id) {
            tx.send(response).map_err(|_| "send failed".to_string())
        } else {
            Err("no request found".to_string())
        }
    }

    fn respond_to_rust_storage(request_id: u64, response: Vec<u8>) -> Result<(), String> {
        let mut map = REQ_REGISTRY_STORAGE.lock().unwrap();

        if let Some(tx) = map.remove(&request_id) {
            tx.send(response).map_err(|_| "send failed".to_string())
        } else {
            Err("no request found".to_string())
        }
    }

    fn respond_to_rust_call(
        request_id: u64,
        main_error: Vec<u8>,
        logs: Vec<Vec<u8>>,
        exec_cost: u64,
        result: Option<Vec<u8>>,
    ) -> Result<(), String> {
        let mut map = REQ_REGISTRY_CALL.lock().unwrap();

        if let Some(tx) = map.remove(&request_id) {
            tx.send((main_error, logs, exec_cost, result))
                .map_err(|_| "send failed".to_string())
        } else {
            Err("no request found".to_string())
        }
    }
}
