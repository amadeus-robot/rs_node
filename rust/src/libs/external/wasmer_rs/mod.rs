use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::Duration;
use sha2::{Sha256, Digest};
use rand::random;

// Module cache for compiled artifacts
static MODULE_CACHE: OnceLock<Mutex<HashMap<[u8; 32], Arc<String>>>> = OnceLock::new();

// Request registries for async operations
static REQ_REGISTRY_STORAGE_KV_GET: OnceLock<Mutex<HashMap<u64, mpsc::Sender<Option<Vec<u8>>>>>> = OnceLock::new();
static REQ_REGISTRY_STORAGE_KV_EXISTS: OnceLock<Mutex<HashMap<u64, mpsc::Sender<bool>>>> = OnceLock::new();
static REQ_REGISTRY_STORAGE_KV_GET_PREV_NEXT: OnceLock<Mutex<HashMap<u64, mpsc::Sender<(Option<Vec<u8>>, Option<Vec<u8>>)>>>> = OnceLock::new();
static REQ_REGISTRY_STORAGE: OnceLock<Mutex<HashMap<u64, mpsc::Sender<Vec<u8>>>>> = OnceLock::new();
static REQ_REGISTRY_CALL: OnceLock<Mutex<HashMap<u64, mpsc::Sender<(Vec<u8>, Vec<Vec<u8>>, u64, Option<Vec<u8>>)>>>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct ExitCode(u32);

#[derive(Clone)]
struct HostEnv {
    readonly: bool,
    error: Option<Vec<u8>>,
    return_value: Option<Vec<u8>>,
    logs: Vec<Vec<u8>>,
    current_account: Vec<u8>,
    attached_symbol: Vec<u8>,
    attached_amount: Vec<u8>,
}

// Storage KV Get operations
fn request_from_storage_kv_get(reply_to: std::thread::ThreadId, key: Vec<u8>) -> (mpsc::Receiver<Option<Vec<u8>>>, u64) {
    let (tx, rx) = mpsc::channel::<Option<Vec<u8>>>();
    let request_id = random::<u64>();
    {
        let mut map = REQ_REGISTRY_STORAGE_KV_GET.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
        map.insert(request_id, tx);
    }
    (rx, request_id)
}

fn respond_to_storage_kv_get(request_id: u64, response: Option<Vec<u8>>) -> Result<(), String> {
    let mut map = REQ_REGISTRY_STORAGE_KV_GET.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    if let Some(tx) = map.remove(&request_id) {
        tx.send(response).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("no_request_found".to_string())
    }
}

// Storage KV Exists operations
fn request_from_storage_kv_exists(reply_to: std::thread::ThreadId, key: Vec<u8>) -> (mpsc::Receiver<bool>, u64) {
    let (tx, rx) = mpsc::channel::<bool>();
    let request_id = random::<u64>();
    {
        let mut map = REQ_REGISTRY_STORAGE_KV_EXISTS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
        map.insert(request_id, tx);
    }
    (rx, request_id)
}

fn respond_to_storage_kv_exists(request_id: u64, response: bool) -> Result<(), String> {
    let mut map = REQ_REGISTRY_STORAGE_KV_EXISTS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    if let Some(tx) = map.remove(&request_id) {
        tx.send(response).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("no_request_found".to_string())
    }
}

// Storage KV Get Previous/Next operations
fn request_from_storage_kv_get_prev_next(reply_to: std::thread::ThreadId, suffix: Vec<u8>, key: Vec<u8>) -> (mpsc::Receiver<(Option<Vec<u8>>, Option<Vec<u8>>)>, u64) {
    let (tx, rx) = mpsc::channel::<(Option<Vec<u8>>, Option<Vec<u8>>)>();
    let request_id = random::<u64>();
    {
        let mut map = REQ_REGISTRY_STORAGE_KV_GET_PREV_NEXT.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
        map.insert(request_id, tx);
    }
    (rx, request_id)
}

fn respond_to_storage_kv_get_prev_next(request_id: u64, response: (Option<Vec<u8>>, Option<Vec<u8>>)) -> Result<(), String> {
    let mut map = REQ_REGISTRY_STORAGE_KV_GET_PREV_NEXT.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    if let Some(tx) = map.remove(&request_id) {
        tx.send(response).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("no_request_found".to_string())
    }
}

// General storage operations
fn request_from_storage(reply_to: std::thread::ThreadId, data: Vec<u8>) -> (mpsc::Receiver<Vec<u8>>, u64) {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let request_id = random::<u64>();
    {
        let mut map = REQ_REGISTRY_STORAGE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
        map.insert(request_id, tx);
    }
    (rx, request_id)
}

fn respond_to_storage(request_id: u64, response: Vec<u8>) -> Result<(), String> {
    let mut map = REQ_REGISTRY_STORAGE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    if let Some(tx) = map.remove(&request_id) {
        tx.send(response).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("no_request_found".to_string())
    }
}

// Call operations
fn request_from_call(reply_to: std::thread::ThreadId, remaining_points: u64, module: Vec<u8>, function: Vec<u8>, args: Vec<Vec<u8>>, attached_symbol: Vec<u8>, attached_amount: Vec<u8>) -> (mpsc::Receiver<(Vec<u8>, Vec<Vec<u8>>, u64, Option<Vec<u8>>)>, u64) {
    let (tx, rx) = mpsc::channel::<(Vec<u8>, Vec<Vec<u8>>, u64, Option<Vec<u8>>)>();
    let request_id = random::<u64>();
    {
        let mut map = REQ_REGISTRY_CALL.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
        map.insert(request_id, tx);
    }
    (rx, request_id)
}

fn respond_to_call(request_id: u64, main_error: Vec<u8>, logs: Vec<Vec<u8>>, exec_cost: u64, result: Option<Vec<u8>>) -> Result<(), String> {
    let mut map = REQ_REGISTRY_CALL.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    if let Some(tx) = map.remove(&request_id) {
        tx.send((main_error, logs, exec_cost, result)).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("no_request_found".to_string())
    }
}

// Utility functions
fn build_prefixed_key(prefix: &[u8], body: &[u8]) -> Vec<u8> {
    const CONTRACT: &[u8] = b"c:";
    let mut out = Vec::with_capacity(CONTRACT.len() + prefix.len() + 1 + body.len());
    out.extend_from_slice(CONTRACT);
    out.extend_from_slice(prefix);
    out.push(b':');
    out.extend_from_slice(body);
    out
}

fn cost_function(_operator: &()) -> u64 {
    10
}

fn get_or_cache_module(data: &[u8]) -> Result<Arc<String>, String> {
    let cache_mutex = MODULE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize().into();
    
    {
        let cache = cache_mutex.lock().unwrap();
        if let Some(cached_module) = cache.get(&hash) {
            return Ok(Arc::clone(cached_module));
        }
    }
    
    let module_data = Arc::new(String::from_utf8(data.to_vec()).map_err(|e| e.to_string())?);
    
    {
        let mut cache = cache_mutex.lock().unwrap();
        cache.insert(hash, Arc::clone(&module_data));
    }
    
    Ok(module_data)
}

// Main execution function
fn execute_operation(
    data: &[u8],
    operation: &str,
    args: Vec<Vec<u8>>,
    env: HostEnv
) -> Result<(Option<Vec<u8>>, Vec<Vec<u8>>, u64), String> {
    // Get or cache the module
    let _module = get_or_cache_module(data)?;
    
    // Simulate operation execution
    let result = match operation {
        "kv_get" if args.len() == 1 => {
            let key = build_prefixed_key(&env.current_account, &args[0]);
            let (rx, request_id) = request_from_storage_kv_get(std::thread::current().id(), key);
            match rx.recv_timeout(Duration::from_secs(6)) {
                Ok(response) => response,
                Err(_) => {
                    let mut map = REQ_REGISTRY_STORAGE_KV_GET.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
                    map.remove(&request_id);
                    return Err("timeout".to_string());
                }
            }
        }
        "kv_put" if args.len() == 2 => {
            let key = build_prefixed_key(&env.current_account, &args[0]);
            let (rx, request_id) = request_from_storage(std::thread::current().id(), key);
            match rx.recv_timeout(Duration::from_secs(6)) {
                Ok(response) => Some(response),
                Err(_) => {
                    let mut map = REQ_REGISTRY_STORAGE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
                    map.remove(&request_id);
                    return Err("timeout".to_string());
                }
            }
        }
        _ => None,
    };
    
    Ok((result, env.logs, 1000)) // Simulated execution cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prefixed_key() {
        let prefix = b"account123";
        let body = b"key456";
        let result = build_prefixed_key(prefix, body);
        assert_eq!(result, b"c:account123:key456");
    }

    #[test]
    fn test_module_caching() {
        let data = b"test_data";
        let result1 = get_or_cache_module(data).unwrap();
        let result2 = get_or_cache_module(data).unwrap();
        assert!(Arc::ptr_eq(&result1, &result2));
    }
}