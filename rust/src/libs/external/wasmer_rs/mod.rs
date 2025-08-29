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

#[derive(Debug, Clone, Copy)]
struct ExitCode(u32);

#[derive(Clone)]
//struct HostEnv<'a> {
struct HostEnv {
    memory: Option<Memory>,
    readonly: bool,
    error: Option<Vec<u8>>,
    return_value: Option<Vec<u8>>,
    logs: Vec<Vec<u8>>,
    current_account: Vec<u8>,
    // rpc_pid: u8,
    //env: Env<'a>
    instance: Option<Arc<Instance>>,

    attached_symbol: Vec<u8>,
    attached_amount: Vec<u8>,
}
//unsafe impl Sync for HostEnv<'_> {}
//unsafe impl Send for HostEnv<'_> {}

//RPC LOL
/// Respond to a KV get request by sending the response through the stored channel.
pub fn respond_to_rust_storage_kv_get(
    request_id: u64,
    response: Option<Vec<u8>>,
) -> Result<&'static str, &'static str> {
    let mut map = REQ_REGISTRY_STORAGE_KV_GET.lock().unwrap();

    if let Some(tx) = map.remove(&request_id) {
        if tx.send(response).is_err() {
            // The receiver was dropped
            return Err("receiver_dropped");
        }
        Ok("ok")
    } else {
        Err("no_request_found")
    }
}

pub fn respond_to_rust_storage_kv_exists(
    request_id: u64,
    response: bool,
) -> Result<&'static str, &'static str> {
    let mut map = REQ_REGISTRY_STORAGE_KV_EXISTS.lock().unwrap();

    if let Some(tx) = map.remove(&request_id) {
        if tx.send(response).is_err() {
            // The receiver was dropped
            return Err("receiver_dropped");
        }
        Ok("ok")
    } else {
        Err("no_request_found")
    }
}

pub fn respond_to_rust_storage_kv_get_prev_next(
    request_id: u64,
    response: (Option<Vec<u8>>, Option<Vec<u8>>),
) -> Result<&'static str, &'static str> {
    let mut map = REQ_REGISTRY_STORAGE_KV_GET_PREV_NEXT.lock().unwrap();

    if let Some(tx) = map.remove(&request_id) {
        if tx.send(response).is_err() {
            // The receiver was dropped
            return Err("receiver_dropped");
        }
        Ok("ok")
    } else {
        Err("no_request_found")
    }
}

pub fn respond_to_rust_storage(
    request_id: u64,
    response: Vec<u8>,
) -> Result<&'static str, &'static str> {
    let mut map = REQ_REGISTRY_STORAGE.lock().unwrap();

    if let Some(tx) = map.remove(&request_id) {
        if tx.send(response).is_err() {
            // The receiver was dropped
            return Err("receiver_dropped");
        }
        Ok("ok")
    } else {
        Err("no_request_found")
    }
}

pub fn respond_to_rust_call(
    request_id: u64,
    main_error: Vec<u8>,
    logs: Vec<Vec<u8>>,
    exec_cost: u64,
    result: Option<Vec<u8>>,
) -> Result<&'static str, &'static str> {
    let mut map = REQ_REGISTRY_CALL.lock().unwrap();

    if let Some(tx) = map.remove(&request_id) {
        if tx.send((main_error, logs, exec_cost, result)).is_err() {
            return Err("receiver_dropped");
        }
        Ok("ok")
    } else {
        Err("no_request_found")
    }
}

#[inline]
fn charge_points<S>(store: &mut S, instance: &Instance, cost: u64) -> Result<u64, RuntimeError>
where
    S: AsStoreMut,
{
    let remaining = match get_remaining_points(store, instance) {
        MeteringPoints::Remaining(v) => v,
        MeteringPoints::Exhausted => 0,
    };

    if cost > remaining {
        return Err(RuntimeError::new("unreachable"));
    }
    let new_remaining = remaining - cost;
    set_remaining_points(store, instance, new_remaining);
    Ok(new_remaining)
}

fn build_prefixed_key(
    view: &MemoryView,
    prefix: &[u8],
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>, RuntimeError> {
    const CONTRACT: &[u8] = b"c:";

    let mut body = vec![0u8; len as usize];
    view.read(ptr as u64, &mut body)
        .map_err(|_| RuntimeError::new("invalid_memory"))?;

    let mut out = Vec::with_capacity(CONTRACT.len() + prefix.len() + 1 + body.len());
    out.extend_from_slice(CONTRACT);
    out.extend_from_slice(prefix);
    out.push(b':');
    out.extend_from_slice(&body);
    Ok(out)
}

#[inline]
fn write_i32(view: &MemoryView, offset: u64, value: i32) -> Result<(), RuntimeError> {
    view.write(offset, &value.to_le_bytes())
        .map_err(|_| RuntimeError::new("invalid_memory"))
}

#[inline]
fn write_bin(view: &MemoryView, offset: u64, slice: &[u8]) -> Result<(), RuntimeError> {
    view.write(offset, slice)
        .map_err(|_| RuntimeError::new("invalid_memory"))
}

//AssemblyScript specific
fn abort_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    msg_ptr: i32,
    filename_ptr: i32,
    line: i32,
    column: i32,
) -> Result<(), RuntimeError> {
    let (data, mut store) = env.data_and_store_mut();
    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    //I kill thee
    let mut msg_size_bytes = [0u8; 4];
    let Ok(_) = view.read((msg_ptr as u64) - 4, &mut msg_size_bytes) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let msg_size: i32 = i32::from_le_bytes(msg_size_bytes);
    let mut msg_buff_utf16 = vec![0u8; msg_size as usize];
    let Ok(_) = view.read(msg_ptr as u64, &mut msg_buff_utf16) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let msg_buff_utf16_b4collect = msg_buff_utf16
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));
    let msg_buff_utf16_collected: Vec<u16> = msg_buff_utf16_b4collect.collect();

    let mut filename_size_bytes = [0u8; 4];
    let Ok(_) = view.read((filename_ptr as u64) - 4, &mut filename_size_bytes) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let filename_size: i32 = i32::from_le_bytes(filename_size_bytes);
    let mut filename_buff_utf16 = vec![0u8; filename_size as usize];
    let Ok(_) = view.read(filename_ptr as u64, &mut filename_buff_utf16) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let filename_buff_utf16_b4collect = filename_buff_utf16
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));
    let filename_buff_utf16_collected: Vec<u16> = filename_buff_utf16_b4collect.collect();

    let msg_utf8 = match String::from_utf16(&msg_buff_utf16_collected) {
        Ok(s) => s,
        Err(_) => {
            return Err(RuntimeError::new("invalid_memory"));
        }
    };
    let filename_utf8 = match String::from_utf16(&filename_buff_utf16_collected) {
        Ok(s) => s,
        Err(_) => {
            return Err(RuntimeError::new("invalid_memory"));
        }
    };

    let formatted = format!("{} | {} {} {}", msg_utf8, filename_utf8, line, column);
    data.return_value = Some(formatted.into_bytes());

    //println!("{} {} {} {}", msg_utf8, filename_utf8, line, column);

    Err(RuntimeError::new("abort"))
}

fn import_log_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    ptr: i32,
    len: i32,
) -> Result<(), RuntimeError> {
    let cost = (len as u64) * 1000;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut buffer = vec![0u8; len as usize];
    match view.read(ptr as u64, &mut buffer) {
        Ok(_) => {
            //print!("log>> {} \n", String::from_utf8_lossy(&buffer));
            data.logs.push(buffer);
            Ok(())
        }
        Err(read_err) => Err(RuntimeError::new("invalid_memory")),
    }
}

fn import_attach_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    symbol_ptr: i32,
    symbol_len: i32,
    amount_ptr: i32,
    amount_len: i32,
) -> Result<(), RuntimeError> {
    let cost = 1000 + (symbol_len as u64) * 100 + (amount_len as u64) * 100;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut symbol_buffer = vec![0u8; symbol_len as usize];
    let Ok(_) = view.read(symbol_ptr as u64, &mut symbol_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut amount_buffer = vec![0u8; amount_len as usize];
    let Ok(_) = view.read(amount_ptr as u64, &mut amount_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    data.attached_symbol = symbol_buffer;
    data.attached_amount = amount_buffer;
    Ok(())
}

fn import_return_value_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    ptr: i32,
    len: i32,
) -> Result<(), RuntimeError> {
    let cost = (len as u64) * 1000;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut buffer = vec![0u8; len as usize];
    match view.read(ptr as u64, &mut buffer) {
        Ok(_) => {
            //println!("return was {}", String::from_utf8_lossy(&buffer));
            data.return_value = Some(buffer);
            Err(RuntimeError::new("return_value"))
        }
        Err(read_err) => Err(RuntimeError::new("invalid_memory")),
    }
}

pub fn request_from_rust_storage_kv_get(key: Vec<u8>) -> (mpsc::Receiver<Option<Vec<u8>>>, u64) {
    let (tx, rx) = mpsc::channel::<Option<Vec<u8>>>();
    let request_id: u64 = rand::rng().r#gen();

    // store sender in registry
    {
        let mut map = REQ_REGISTRY_STORAGE_KV_GET.lock().unwrap();
        map.insert(request_id, tx);
    }

    // simulate async request handling (e.g., Wasmer module or storage thread)
    std::thread::spawn(move || {
        // Here you would normally call the storage backend.
        // For demonstration, let's just simulate a "found" value:
        let simulated_value: Option<Vec<u8>> = Some(key.clone());

        // send the response back using the stored sender
        if let Some(tx) = REQ_REGISTRY_STORAGE_KV_GET
            .lock()
            .unwrap()
            .remove(&request_id)
        {
            let _ = tx.send(simulated_value);
        }
    });

    (rx, request_id)
}

fn import_storage_kv_get_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32,
    key_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64)) * 100;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let key_buffer = build_prefixed_key(&view, &data.current_account, key_ptr, key_len)?;

    //let mut key_buffer_suffix = vec![0u8; key_len as usize];
    //let Ok(_) = view.read(key_ptr as u64, &mut key_buffer_suffix) else { return Err(RuntimeError::new("invalid_memory")) };
    //let mut key_buffer = data.current_account.clone();
    //key_buffer.extend_from_slice(b":");
    //key_buffer.extend_from_slice(&key_buffer_suffix);

    let (rx, request_id) = request_from_rust_storage_kv_get(key_buffer);

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok(option_response) => match option_response {
            Some(response) => {
                write_i32(&view, 30_000, response.len() as i32)?;
                write_bin(&view, 30_004, &response)?;
                Ok(30_000)
            }
            None => {
                write_i32(&view, 30_000, -1)?;
                Ok(30_000)
            }
        },
        Err(_) => {
            let mut map = REQ_REGISTRY_STORAGE_KV_GET.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}

///EXISTS
fn request_from_rust_storage_kv_exists(key: Vec<u8>) -> (std::sync::mpsc::Receiver<bool>, u64) {
    let (tx, rx) = std::sync::mpsc::channel::<bool>();
    let request_id = rand::random::<u64>();

    {
        let mut map = REQ_REGISTRY_STORAGE_KV_EXISTS.lock().unwrap();
        map.insert(request_id, tx);
    }

    // No need for thread spawn with OwnedEnv
    (rx, request_id)
}

fn import_storage_kv_exists_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32,
    key_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64)) * 100;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let key_buffer = build_prefixed_key(&view, &data.current_account, key_ptr, key_len)?;
    /*
        let mut key_buffer_suffix = vec![0u8; key_len as usize];
        let Ok(_) = view.read(key_ptr as u64, &mut key_buffer_suffix) else { return Err(RuntimeError::new("invalid_memory")) };
        let mut key_buffer = data.current_account.clone();
        key_buffer.extend_from_slice(b":");
        key_buffer.extend_from_slice(&key_buffer_suffix);
    */
    let (rx, request_id) = request_from_rust_storage_kv_exists(key_buffer);

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok(true) => {
            write_i32(&view, 30_000, 1)?;
            Ok(30_000)
        }
        Ok(false) => {
            write_i32(&view, 30_000, 0)?;
            Ok(30_000)
        }
        Err(_) => {
            let mut map = REQ_REGISTRY_STORAGE_KV_EXISTS.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}

///PREV
fn request_from_rust_storage_kv_get_prev(
    send_request_fn: impl Fn(u64, Vec<u8>, Vec<u8>) + Send + 'static,
    suffix: Vec<u8>,
    key: Vec<u8>,
) -> (
    std::sync::mpsc::Receiver<(Option<Vec<u8>>, Option<Vec<u8>>)>,
    u64,
) {
    let (tx, rx) = std::sync::mpsc::channel::<(Option<Vec<u8>>, Option<Vec<u8>>)>();
    let request_id = rand::random::<u64>();

    {
        let mut map = REQ_REGISTRY_STORAGE_KV_GET_PREV_NEXT.lock().unwrap();
        map.insert(request_id, tx);
    }

    std::thread::spawn(move || {
        send_request_fn(request_id, suffix, key);
    });

    (rx, request_id)
}

fn import_storage_kv_get_prev_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    suffix_ptr: i32,
    suffix_len: i32,
    key_ptr: i32,
    key_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64)) * 100;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let suffix_buffer = build_prefixed_key(&view, &data.current_account, suffix_ptr, suffix_len)?;

    /*
        let mut suffix_buffer_suffix = vec![0u8; suffix_len as usize];
        let Ok(_) = view.read(suffix_ptr as u64, &mut suffix_buffer_suffix) else { return Err(RuntimeError::new("invalid_memory")) };
        let mut suffix_buffer = data.current_account.clone();
        suffix_buffer.extend_from_slice(b":");
        suffix_buffer.extend_from_slice(&suffix_buffer_suffix);
    */

    let mut key_buffer = vec![0u8; key_len as usize];
    let Ok(_) = view.read(key_ptr as u64, &mut key_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let (rx, request_id) =
        request_from_rust_storage_kv_get_prev(data.rpc_pid, suffix_buffer, key_buffer);

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok((maybe_prev_key, maybe_value)) => match (maybe_prev_key, maybe_value) {
            (Some(prev_key), Some(value)) => {
                write_i32(&view, 30_000, prev_key.len() as i32)?;
                write_bin(&view, 30_004, &prev_key)?;

                write_i32(&view, 30_004 + (prev_key.len() as u64), value.len() as i32)?;
                write_bin(&view, 30_004 + (prev_key.len() as u64) + 4, &value)?;
                Ok(30_000)
            }
            _ => {
                write_i32(&view, 30_000, -1)?;
                Ok(30_000)
            }
        },
        Err(_) => {
            let mut map = REQ_REGISTRY_STORAGE_KV_GET_PREV_NEXT.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}

///NEXT
fn request_from_rust_storage_kv_get_next(
    reply_to_pid: LocalPid,
    suffix: Vec<u8>,
    key: Vec<u8>,
) -> (
    std::sync::mpsc::Receiver<(Option<Vec<u8>>, Option<Vec<u8>>)>,
    u64,
) {
    let (tx, rx) = mpsc::channel::<(Option<Vec<u8>>, Option<Vec<u8>>)>();
    let request_id = rand::random::<u64>();
    {
        let mut map = REQ_REGISTRY_STORAGE_KV_GET_PREV_NEXT.lock().unwrap();
        map.insert(request_id, tx);
    }

    std::thread::spawn(move || {
        let mut env = OwnedEnv::new();
        let _ = env.send_and_clear(&reply_to_pid, |cenv| {
            let mut owned_suffix = OwnedBinary::new(suffix.len()).unwrap();
            owned_suffix.as_mut_slice().copy_from_slice(&suffix);
            let mut owned_key = OwnedBinary::new(key.len()).unwrap();
            owned_key.as_mut_slice().copy_from_slice(&key);
            let payload = (
                atoms::rust_request_storage_kv_get_next(),
                request_id,
                Binary::from_owned(owned_suffix, cenv),
                Binary::from_owned(owned_key, cenv),
            );
            payload.encode(cenv)
        });
    });

    (rx, request_id)
}
fn import_storage_kv_get_next_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    suffix_ptr: i32,
    suffix_len: i32,
    key_ptr: i32,
    key_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64)) * 100;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let suffix_buffer = build_prefixed_key(&view, &data.current_account, suffix_ptr, suffix_len)?;
    /*
        let mut suffix_buffer_suffix = vec![0u8; suffix_len as usize];
        let Ok(_) = view.read(suffix_ptr as u64, &mut suffix_buffer_suffix) else { return Err(RuntimeError::new("invalid_memory")) };
        let mut suffix_buffer = data.current_account.clone();
        suffix_buffer.extend_from_slice(b":");
        suffix_buffer.extend_from_slice(&suffix_buffer_suffix);
    */
    let mut key_buffer = vec![0u8; key_len as usize];
    let Ok(_) = view.read(key_ptr as u64, &mut key_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let (rx, request_id) =
        request_from_rust_storage_kv_get_next(data.rpc_pid, suffix_buffer, key_buffer);

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok((maybe_next_key, maybe_value)) => match (maybe_next_key, maybe_value) {
            (Some(next_key), Some(value)) => {
                write_i32(&view, 30_000, next_key.len() as i32)?;
                write_bin(&view, 30_004, &next_key)?;
                write_i32(&view, 30_004 + (next_key.len() as u64), value.len() as i32)?;
                write_bin(&view, 30_004 + (next_key.len() as u64) + 4, &value)?;
                Ok(30_000)
            }
            _ => {
                write_i32(&view, 30_000, -1)?;
                Ok(30_000)
            }
        },
        Err(_) => {
            let mut map = REQ_REGISTRY_STORAGE_KV_GET_PREV_NEXT.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}

//PUT
fn request_from_rust_storage_kv_put<'a>(
    reply_to_pid: LocalPid,
    key: Vec<u8>,
    val: Vec<u8>,
) -> (std::sync::mpsc::Receiver<Vec<u8>>, u64) {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let request_id = rand::random::<u64>();
    {
        let mut map = REQ_REGISTRY_STORAGE.lock().unwrap();
        map.insert(request_id, tx);
    }

    std::thread::spawn(move || {
        let mut env = OwnedEnv::new();
        let _ = env.send_and_clear(&reply_to_pid, |cenv| {
            let mut owned_key = OwnedBinary::new(key.len()).unwrap();
            owned_key.as_mut_slice().copy_from_slice(&key);
            let mut owned_val = OwnedBinary::new(val.len()).unwrap();
            owned_val.as_mut_slice().copy_from_slice(&val);
            let payload = (
                atoms::rust_request_storage_kv_put(),
                request_id,
                Binary::from_owned(owned_key, cenv),
                Binary::from_owned(owned_val, cenv),
            );
            payload.encode(cenv)
        });
    });

    (rx, request_id)
}
fn import_storage_kv_put_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32,
    key_len: i32,
    val_ptr: i32,
    val_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64) + (val_len as u64)) * 1000;

    let (data, mut store) = env.data_and_store_mut();
    if data.readonly {
        return Err(RuntimeError::new("read_only"));
    }

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let key_buffer = build_prefixed_key(&view, &data.current_account, key_ptr, key_len)?;
    /*
       let mut key_buffer_suffix = vec![0u8; key_len as usize];
       let Ok(_) = view.read(key_ptr as u64, &mut key_buffer_suffix) else { return Err(RuntimeError::new("invalid_memory")) };
       let mut key_buffer = data.current_account.clone();
       key_buffer.extend_from_slice(b":");
       key_buffer.extend_from_slice(&key_buffer_suffix);
    */
    let mut val_buffer = vec![0u8; val_len as usize];
    let Ok(_) = view.read(val_ptr as u64, &mut val_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let (rx, request_id) = request_from_rust_storage_kv_put(data.rpc_pid, key_buffer, val_buffer);

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok(response) => {
            write_i32(&view, 30_000, response.len() as i32)?;
            write_bin(&view, 30_004, &response)?;
            Ok(30_000)
        }
        Err(_) => {
            let mut map = REQ_REGISTRY_STORAGE.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}

//INCREMENT
fn request_from_rust_storage_kv_increment<'a>(
    reply_to_pid: LocalPid,
    key: Vec<u8>,
    val: Vec<u8>,
) -> (std::sync::mpsc::Receiver<Vec<u8>>, u64) {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let request_id = rand::random::<u64>();
    {
        let mut map = REQ_REGISTRY_STORAGE.lock().unwrap();
        map.insert(request_id, tx);
    }

    std::thread::spawn(move || {
        let mut env = OwnedEnv::new();
        let _ = env.send_and_clear(&reply_to_pid, |cenv| {
            let mut owned_key = OwnedBinary::new(key.len()).unwrap();
            owned_key.as_mut_slice().copy_from_slice(&key);
            let mut owned_val = OwnedBinary::new(val.len()).unwrap();
            owned_val.as_mut_slice().copy_from_slice(&val);
            let payload = (
                atoms::rust_request_storage_kv_increment(),
                request_id,
                Binary::from_owned(owned_key, cenv),
                Binary::from_owned(owned_val, cenv),
            );
            payload.encode(cenv)
        });
    });

    (rx, request_id)
}
fn import_storage_kv_increment_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32,
    key_len: i32,
    val_ptr: i32,
    val_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64) + (val_len as u64)) * 1000;

    let (data, mut store) = env.data_and_store_mut();
    if data.readonly {
        return Err(RuntimeError::new("read_only"));
    }

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let key_buffer = build_prefixed_key(&view, &data.current_account, key_ptr, key_len)?;
    /*
        let mut key_buffer_suffix = vec![0u8; key_len as usize];
        let Ok(_) = view.read(key_ptr as u64, &mut key_buffer_suffix) else { return Err(RuntimeError::new("invalid_memory")) };
        let mut key_buffer = data.current_account.clone();
        key_buffer.extend_from_slice(b":");
        key_buffer.extend_from_slice(&key_buffer_suffix);
    */
    let mut val_buffer = vec![0u8; val_len as usize];
    let Ok(_) = view.read(val_ptr as u64, &mut val_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let (rx, request_id) =
        request_from_rust_storage_kv_increment(data.rpc_pid, key_buffer, val_buffer);

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok(response) => {
            write_i32(&view, 30_000, response.len() as i32)?;
            write_bin(&view, 30_004, &response)?;
            Ok(30_000)
        }
        Err(_) => {
            let mut map = REQ_REGISTRY_STORAGE.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}

//DELETE
fn request_from_rust_storage_kv_delete<'a>(
    reply_to_pid: LocalPid,
    key: Vec<u8>,
) -> (std::sync::mpsc::Receiver<Vec<u8>>, u64) {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let request_id = rand::random::<u64>();
    {
        let mut map = REQ_REGISTRY_STORAGE.lock().unwrap();
        map.insert(request_id, tx);
    }

    std::thread::spawn(move || {
        let mut env = OwnedEnv::new();
        let _ = env.send_and_clear(&reply_to_pid, |cenv| {
            let mut owned_key = OwnedBinary::new(key.len()).unwrap();
            owned_key.as_mut_slice().copy_from_slice(&key);
            let payload = (
                atoms::rust_request_storage_kv_delete(),
                request_id,
                Binary::from_owned(owned_key, cenv),
            );
            payload.encode(cenv)
        });
    });

    (rx, request_id)
}
fn import_storage_kv_delete_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32,
    key_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64)) * 1000;

    let (data, mut store) = env.data_and_store_mut();
    if data.readonly {
        return Err(RuntimeError::new("read_only"));
    }

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let key_buffer = build_prefixed_key(&view, &data.current_account, key_ptr, key_len)?;
    /*
        let mut key_buffer_suffix = vec![0u8; key_len as usize];
        let Ok(_) = view.read(key_ptr as u64, &mut key_buffer_suffix) else { return Err(RuntimeError::new("invalid_memory")) };
        let mut key_buffer = data.current_account.clone();
        key_buffer.extend_from_slice(b":");
        key_buffer.extend_from_slice(&key_buffer_suffix);
    */
    let (rx, request_id) = request_from_rust_storage_kv_delete(data.rpc_pid, key_buffer);
    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok(response) => {
            write_i32(&view, 30_000, response.len() as i32)?;
            write_bin(&view, 30_004, &response)?;
            Ok(30_000)
        }
        Err(_) => {
            let mut map = REQ_REGISTRY_STORAGE.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}

//CLEAR
fn request_from_rust_storage_kv_clear<'a>(
    reply_to_pid: LocalPid,
    prefix: Vec<u8>,
) -> (std::sync::mpsc::Receiver<Vec<u8>>, u64) {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let request_id = rand::random::<u64>();
    {
        let mut map = REQ_REGISTRY_STORAGE.lock().unwrap();
        map.insert(request_id, tx);
    }

    std::thread::spawn(move || {
        let mut env = OwnedEnv::new();
        let _ = env.send_and_clear(&reply_to_pid, |cenv| {
            let mut owned_prefix = OwnedBinary::new(prefix.len()).unwrap();
            owned_prefix.as_mut_slice().copy_from_slice(&prefix);
            let payload = (
                atoms::rust_request_storage_kv_delete(),
                request_id,
                Binary::from_owned(owned_prefix, cenv),
            );
            payload.encode(cenv)
        });
    });

    (rx, request_id)
}
fn import_storage_kv_clear_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    prefix_ptr: i32,
    prefix_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (prefix_len as u64)) * 1000;

    let (data, mut store) = env.data_and_store_mut();
    if data.readonly {
        return Err(RuntimeError::new("read_only"));
    }

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let prefix_buffer = build_prefixed_key(&view, &data.current_account, prefix_ptr, prefix_len)?;
    /*
        let mut prefix_buffer_suffix = vec![0u8; prefix_len as usize];
        let Ok(_) = view.read(prefix_ptr as u64, &mut prefix_buffer_suffix) else { return Err(RuntimeError::new("invalid_memory")) };
        let mut prefix_buffer = data.current_account.clone();
        prefix_buffer.extend_from_slice(b":");
        prefix_buffer.extend_from_slice(&prefix_buffer_suffix);
    */
    let (rx, request_id) = request_from_rust_storage_kv_clear(data.rpc_pid, prefix_buffer);
    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok(response) => {
            write_i32(&view, 30_000, response.len() as i32)?;
            write_bin(&view, 30_004, &response)?;
            Ok(30_000)
        }
        Err(_) => {
            let mut map = REQ_REGISTRY_STORAGE.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}

//CALL
fn request_from_rust_call<'a>(
    reply_to_pid: LocalPid,
    remaining_u64: u64,
    module: Vec<u8>,
    function: Vec<u8>,
    args: Vec<Vec<u8>>,
    attached_symbol: Vec<u8>,
    attached_amount: Vec<u8>,
) -> (
    std::sync::mpsc::Receiver<(Vec<u8>, Vec<Vec<u8>>, u64, Option<Vec<u8>>)>,
    u64,
) {
    let (tx, rx) = mpsc::channel::<(Vec<u8>, Vec<Vec<u8>>, u64, Option<Vec<u8>>)>();
    let request_id = rand::random::<u64>();
    {
        let mut map = REQ_REGISTRY_CALL.lock().unwrap();
        map.insert(request_id, tx);
    }

    std::thread::spawn(move || {
        let mut env = OwnedEnv::new();
        let _ = env.send_and_clear(&reply_to_pid, |cenv| {
            let mut owned_module = OwnedBinary::new(module.len()).unwrap();
            owned_module.as_mut_slice().copy_from_slice(&module);
            let mut owned_function = OwnedBinary::new(function.len()).unwrap();
            owned_function.as_mut_slice().copy_from_slice(&function);
            let encoded_args: Vec<Binary> = args
                .iter()
                .map(|bytes| {
                    let mut bin = OwnedBinary::new(bytes.len()).unwrap();
                    bin.as_mut_slice().copy_from_slice(bytes);
                    Binary::from_owned(bin, cenv)
                })
                .collect();
            let mut owned_attached_symbol = OwnedBinary::new(attached_symbol.len()).unwrap();
            owned_attached_symbol
                .as_mut_slice()
                .copy_from_slice(&attached_symbol);
            let mut owned_attached_amount = OwnedBinary::new(attached_amount.len()).unwrap();
            owned_attached_amount
                .as_mut_slice()
                .copy_from_slice(&attached_amount);

            let payload = (
                atoms::rust_request_call(),
                request_id,
                remaining_u64,
                Binary::from_owned(owned_module, cenv),
                Binary::from_owned(owned_function, cenv),
                encoded_args,
                (
                    Binary::from_owned(owned_attached_symbol, cenv),
                    Binary::from_owned(owned_attached_amount, cenv),
                ),
            );
            payload.encode(cenv)
        });
    });

    (rx, request_id)
}
fn import_call_0_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    module_ptr: i32,
    module_len: i32,
    function_ptr: i32,
    function_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48) * 1000;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut module_buffer = vec![0u8; module_len as usize];
    let Ok(_) = view.read(module_ptr as u64, &mut module_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut function_buffer = vec![0u8; function_len as usize];
    let Ok(_) = view.read(function_ptr as u64, &mut function_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut args = Vec::with_capacity(0);

    let (rx, request_id) = request_from_rust_call(
        data.rpc_pid,
        remaining_u64,
        module_buffer,
        function_buffer,
        args,
        data.attached_symbol.clone(),
        data.attached_amount.clone(),
    );

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok((error, logs, remaining_exec, result)) => {
            if error != b"ok" {
                return Err(RuntimeError::new("xcc_failed"));
            }

            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            write_i32(&view, 30_000, error.len() as i32)?;
            write_bin(&view, 30_004, &error)?;

            match result {
                Some(bytes) => {
                    write_i32(&view, 30_004 + (error.len() as u64), bytes.len() as i32)?;
                    write_bin(&view, 30_004 + (error.len() as u64) + 4, &bytes)?;
                }
                None => {
                    write_i32(&view, 30_004 + (error.len() as u64), 0)?;
                }
            }

            data.logs.extend(logs);
            set_remaining_points(&mut store, instance_arc.as_ref(), remaining_exec);

            Ok(30_000)
        }
        Err(_) => {
            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            let mut map = REQ_REGISTRY_CALL.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}
fn import_call_1_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    module_ptr: i32,
    module_len: i32,
    function_ptr: i32,
    function_len: i32,
    arg_1_ptr: i32,
    arg_1_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (arg_1_len as u64)) * 1000;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut module_buffer = vec![0u8; module_len as usize];
    let Ok(_) = view.read(module_ptr as u64, &mut module_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut function_buffer = vec![0u8; function_len as usize];
    let Ok(_) = view.read(function_ptr as u64, &mut function_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut arg_1_buffer = vec![0u8; arg_1_len as usize];
    let Ok(_) = view.read(arg_1_ptr as u64, &mut arg_1_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut args = Vec::with_capacity(1);
    args.push(arg_1_buffer);

    let (rx, request_id) = request_from_rust_call(
        data.rpc_pid,
        remaining_u64,
        module_buffer,
        function_buffer,
        args,
        data.attached_symbol.clone(),
        data.attached_amount.clone(),
    );

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok((error, logs, remaining_exec, result)) => {
            if error != b"ok" {
                return Err(RuntimeError::new("xcc_failed"));
            }

            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            write_i32(&view, 30_000, error.len() as i32)?;
            write_bin(&view, 30_004, &error)?;

            match result {
                Some(bytes) => {
                    write_i32(&view, 30_004 + (error.len() as u64), bytes.len() as i32)?;
                    write_bin(&view, 30_004 + (error.len() as u64) + 4, &bytes)?;
                }
                None => {
                    write_i32(&view, 30_004 + (error.len() as u64), 0)?;
                }
            }

            data.logs.extend(logs);
            set_remaining_points(&mut store, instance_arc.as_ref(), remaining_exec);

            Ok(30_000)
        }
        Err(_) => {
            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            let mut map = REQ_REGISTRY_CALL.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}
fn import_call_2_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    module_ptr: i32,
    module_len: i32,
    function_ptr: i32,
    function_len: i32,
    arg_1_ptr: i32,
    arg_1_len: i32,
    arg_2_ptr: i32,
    arg_2_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (arg_1_len as u64) + (arg_2_len as u64)) * 1000;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut module_buffer = vec![0u8; module_len as usize];
    let Ok(_) = view.read(module_ptr as u64, &mut module_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut function_buffer = vec![0u8; function_len as usize];
    let Ok(_) = view.read(function_ptr as u64, &mut function_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut arg_1_buffer = vec![0u8; arg_1_len as usize];
    let Ok(_) = view.read(arg_1_ptr as u64, &mut arg_1_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut arg_2_buffer = vec![0u8; arg_2_len as usize];
    let Ok(_) = view.read(arg_2_ptr as u64, &mut arg_2_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut args = Vec::with_capacity(2);
    args.push(arg_1_buffer);
    args.push(arg_2_buffer);

    let (rx, request_id) = request_from_rust_call(
        data.rpc_pid,
        remaining_u64,
        module_buffer,
        function_buffer,
        args,
        data.attached_symbol.clone(),
        data.attached_amount.clone(),
    );

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok((error, logs, remaining_exec, result)) => {
            if error != b"ok" {
                return Err(RuntimeError::new("xcc_failed"));
            }

            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            write_i32(&view, 30_000, error.len() as i32)?;
            write_bin(&view, 30_004, &error)?;

            match result {
                Some(bytes) => {
                    write_i32(&view, 30_004 + (error.len() as u64), bytes.len() as i32)?;
                    write_bin(&view, 30_004 + (error.len() as u64) + 4, &bytes)?;
                }
                None => {
                    write_i32(&view, 30_004 + (error.len() as u64), 0)?;
                }
            }

            data.logs.extend(logs);
            set_remaining_points(&mut store, instance_arc.as_ref(), remaining_exec);

            Ok(30_000)
        }
        Err(_) => {
            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            let mut map = REQ_REGISTRY_CALL.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}
fn import_call_3_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    module_ptr: i32,
    module_len: i32,
    function_ptr: i32,
    function_len: i32,
    arg_1_ptr: i32,
    arg_1_len: i32,
    arg_2_ptr: i32,
    arg_2_len: i32,
    arg_3_ptr: i32,
    arg_3_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (arg_1_len as u64) + (arg_2_len as u64) + (arg_3_len as u64)) * 1000;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut module_buffer = vec![0u8; module_len as usize];
    let Ok(_) = view.read(module_ptr as u64, &mut module_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut function_buffer = vec![0u8; function_len as usize];
    let Ok(_) = view.read(function_ptr as u64, &mut function_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut arg_1_buffer = vec![0u8; arg_1_len as usize];
    let Ok(_) = view.read(arg_1_ptr as u64, &mut arg_1_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut arg_2_buffer = vec![0u8; arg_2_len as usize];
    let Ok(_) = view.read(arg_2_ptr as u64, &mut arg_2_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut arg_3_buffer = vec![0u8; arg_3_len as usize];
    let Ok(_) = view.read(arg_3_ptr as u64, &mut arg_3_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut args = Vec::with_capacity(3);
    args.push(arg_1_buffer);
    args.push(arg_2_buffer);
    args.push(arg_3_buffer);

    let (rx, request_id) = request_from_rust_call(
        data.rpc_pid,
        remaining_u64,
        module_buffer,
        function_buffer,
        args,
        data.attached_symbol.clone(),
        data.attached_amount.clone(),
    );

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok((error, logs, remaining_exec, result)) => {
            if error != b"ok" {
                return Err(RuntimeError::new("xcc_failed"));
            }

            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            write_i32(&view, 30_000, error.len() as i32)?;
            write_bin(&view, 30_004, &error)?;

            match result {
                Some(bytes) => {
                    write_i32(&view, 30_004 + (error.len() as u64), bytes.len() as i32)?;
                    write_bin(&view, 30_004 + (error.len() as u64) + 4, &bytes)?;
                }
                None => {
                    write_i32(&view, 30_004 + (error.len() as u64), 0)?;
                }
            }

            data.logs.extend(logs);
            set_remaining_points(&mut store, instance_arc.as_ref(), remaining_exec);

            Ok(30_000)
        }
        Err(_) => {
            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            let mut map = REQ_REGISTRY_CALL.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}
fn import_call_4_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    module_ptr: i32,
    module_len: i32,
    function_ptr: i32,
    function_len: i32,
    arg_1_ptr: i32,
    arg_1_len: i32,
    arg_2_ptr: i32,
    arg_2_len: i32,
    arg_3_ptr: i32,
    arg_3_len: i32,
    arg_4_ptr: i32,
    arg_4_len: i32,
) -> Result<i32, RuntimeError> {
    let cost =
        (48 + (arg_1_len as u64) + (arg_2_len as u64) + (arg_3_len as u64) + (arg_4_len as u64))
            * 1000;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut module_buffer = vec![0u8; module_len as usize];
    let Ok(_) = view.read(module_ptr as u64, &mut module_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut function_buffer = vec![0u8; function_len as usize];
    let Ok(_) = view.read(function_ptr as u64, &mut function_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut arg_1_buffer = vec![0u8; arg_1_len as usize];
    let Ok(_) = view.read(arg_1_ptr as u64, &mut arg_1_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut arg_2_buffer = vec![0u8; arg_2_len as usize];
    let Ok(_) = view.read(arg_2_ptr as u64, &mut arg_2_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut arg_3_buffer = vec![0u8; arg_3_len as usize];
    let Ok(_) = view.read(arg_3_ptr as u64, &mut arg_3_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut arg_4_buffer = vec![0u8; arg_4_len as usize];
    let Ok(_) = view.read(arg_4_ptr as u64, &mut arg_4_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut args = Vec::with_capacity(4);
    args.push(arg_1_buffer);
    args.push(arg_2_buffer);
    args.push(arg_3_buffer);
    args.push(arg_4_buffer);

    let (rx, request_id) = request_from_rust_call(
        data.rpc_pid,
        remaining_u64,
        module_buffer,
        function_buffer,
        args,
        data.attached_symbol.clone(),
        data.attached_amount.clone(),
    );

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok((error, logs, remaining_exec, result)) => {
            if error != b"ok" {
                return Err(RuntimeError::new("xcc_failed"));
            }

            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            write_i32(&view, 30_000, error.len() as i32)?;
            write_bin(&view, 30_004, &error)?;

            match result {
                Some(bytes) => {
                    write_i32(&view, 30_004 + (error.len() as u64), bytes.len() as i32)?;
                    write_bin(&view, 30_004 + (error.len() as u64) + 4, &bytes)?;
                }
                None => {
                    write_i32(&view, 30_004 + (error.len() as u64), 0)?;
                }
            }

            data.logs.extend(logs);
            set_remaining_points(&mut store, instance_arc.as_ref(), remaining_exec);

            Ok(30_000)
        }
        Err(_) => {
            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            let mut map = REQ_REGISTRY_CALL.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
}

fn cost_function(operator: &Operator) -> u64 {
    10
    /*match operator {
        Operator::Loop { .. }
        | Operator::Block { .. }
        | Operator::If { .. }
        | Operator::Else { .. }
        | Operator::End { .. }
        | Operator::Br { .. }
        | Operator::BrIf { .. }
        | Operator::Return { .. }
        | Operator::Unreachable { .. } => 1,

        Operator::Call { .. } | Operator::CallIndirect { .. } => 5,

        Operator::I32Load { .. }
        | Operator::I64Load { .. }
        | Operator::F32Load { .. }
        | Operator::F64Load { .. }
        | Operator::I32Store { .. }
        | Operator::I64Store { .. }
        | Operator::F32Store { .. }
        | Operator::F64Store { .. } => 3,

        _ => 2,
    }*/
}

use std::time::{Duration, SystemTime};

/*
fn get_or_compile_module(wasm_bytes: &[u8]) -> Result<(Arc<Engine>, Arc<Module>), rustler::Error> {
    let cache_mutex = MODULE_CACHE.get_or_init(|| {
        Mutex::new(HashMap::new())
    });

    let mut hasher = Sha256::new();
    hasher.update(wasm_bytes);
    let hash = hasher.finalize().into();
    {
        let cache = cache_mutex.lock().unwrap();
        if let Some((cached_engine, cached_module)) = cache.get(&hash) {
            return Ok((Arc::clone(cached_engine), Arc::clone(cached_module)));
        }
    }

    let metering = Arc::new(Metering::new(10_000_000, cost_function));
    let mut compiler = Singlepass::default();
    compiler.canonicalize_nans(true);

    use wasmer::CompilerConfig;
    compiler.push_middleware(metering);

    let mut features = Features::new();
    //features.bulk_memory(false); #required for modern compilers to WASM
    features.threads(false);
    features.reference_types(false);
    features.simd(false);
    features.multi_value(false);
    features.tail_call(false);
    features.module_linking(false);
    features.multi_memory(false);
    features.memory64(false);

    //let engine = EngineBuilder::new(compiler).set_features(Some(features));
    //let mut store = Store::new(engine);

    let engine = Arc::new(EngineBuilder::new(compiler).set_features(Some(features)));
    let store = Store::new(Arc::clone(&engine));

    let module = Module::new(&store, &wasm_bytes).map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;

    let arc_module = Arc::new(module);
    {
        let mut cache = cache_mutex.lock().unwrap();
        cache.insert(hash, (Arc::clone(&engine), Arc::clone(&arc_module)));
    }

    Ok((engine, arc_module))
}
*/

#[rustler::nif(schedule = "DirtyCpu")]
fn call<'a>(
    env: Env<'a>,
    rpc_pid: LocalPid,
    mapenv: Term<'a>,
    wasm_bytes: Binary,
    function_name: String,
    function_args: Vec<Term<'a>>,
) -> Result<rustler::Term<'a>, rustler::Error> {
    //TODO: caching
    /*
        let module_arc = get_or_compile_module(wasm_bytes.as_slice());
        let module_ref: &Module = Arc::as_ref(&module_arc);  // Deref the Arc
        let mut store = Store::new(module_ref.engine().clone());
    */

    let exec_points = mapenv
        .map_get(atoms::call_exec_points_remaining())?
        .decode::<u64>()?;
    let metering = Arc::new(Metering::new(exec_points, cost_function));
    let mut compiler = Singlepass::default();
    compiler.canonicalize_nans(true);

    use wasmer::CompilerConfig;
    compiler.push_middleware(metering);

    let mut features = Features::new();
    //features.bulk_memory(false); #required for modern compilers to WASM
    features.threads(false);
    features.reference_types(false);
    features.simd(false);
    features.multi_value(false);
    features.tail_call(false);
    features.module_linking(false);
    features.multi_memory(false);
    features.memory64(false);

    let engine = EngineBuilder::new(compiler).set_features(Some(features));
    let mut store = Store::new(engine);

    let module = Module::new(&store, &wasm_bytes.as_slice())
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;

    //let mut duration_since_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    //println!("ns2 {}", duration_since_epoch.as_nanos());

    let memory = Memory::new(&mut store, MemoryType::new(Pages(8), None, false))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    // memory.view(&mut store).copy_to_memory

    let it1 = mapenv
        .map_get(atoms::seed())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(10_000, &((32 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(10_004, it1)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it2 = mapenv
        .map_get(atoms::entry_signer())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(10_100, &((48 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(10_104, it2)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it3 = mapenv
        .map_get(atoms::entry_prev_hash())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(10_200, &((32 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(10_204, it3)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it4 = mapenv
        .map_get(atoms::entry_vr())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(10_300, &((96 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(10_304, it4)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it5 = mapenv
        .map_get(atoms::entry_dr())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(10_400, &((96 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(10_404, it5)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it6 = mapenv
        .map_get(atoms::tx_signer())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(11_000, &((48 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(11_004, it6)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it7 = mapenv
        .map_get(atoms::account_current())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(12_000, &((it7.len() as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(12_004, it7)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it8 = mapenv
        .map_get(atoms::account_caller())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(13_000, &((it8.len() as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(13_004, it8)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it9 = mapenv
        .map_get(atoms::account_origin())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(14_000, &((it9.len() as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(14_004, it9)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it10 = mapenv
        .map_get(atoms::attached_symbol())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(15_000, &((it10.len() as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(15_004, it10)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it11 = mapenv
        .map_get(atoms::attached_amount())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(16_000, &((it11.len() as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(16_004, it11)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;

    let mut offset: u64 = 20_000;
    let mut wasm_args: Vec<Value> = Vec::with_capacity(function_args.len());

    for term in &function_args {
        if let Ok(i) = term.decode::<i64>() {
            wasm_args.push(Value::I64(i));
        } else if let Ok(b) = term.decode::<Binary>() {
            let bytes = b.as_slice();
            let length = bytes.len();

            memory
                .view(&mut store)
                .write(offset, &(length as i32).to_le_bytes())
                .map_err(|err| Error::Term(Box::new(err.to_string())))?;
            memory
                .view(&mut store)
                .write(offset + 4, bytes)
                .map_err(|err| Error::Term(Box::new(err.to_string())))?;

            let pointer = offset as i32;
            offset += 4 + length as u64;
            wasm_args.push(Value::I32(pointer));
        } else {
            return Err(Error::BadArg);
        }
    }

    let host_env = FunctionEnv::new(
        &mut store,
        HostEnv {
            memory: None,
            error: None,
            return_value: None,
            logs: vec![],
            readonly: mapenv.map_get(atoms::readonly())?.decode::<bool>()?,
            rpc_pid: rpc_pid,
            current_account: it7.to_vec(),
            instance: None,
            attached_symbol: Vec::new(),
            attached_amount: Vec::new(),
        },
    );

    let import_object = imports! {
        "env" => {
            "memory" => memory,
            "seed_ptr" => Global::new(&mut store, Value::I32(10_000)),
            "entry_signer_ptr" => Global::new(&mut store, Value::I32(10_100)),
            "entry_prev_hash_ptr" => Global::new(&mut store, Value::I32(10_200)),
            "entry_slot" => Global::new(&mut store, Value::I64(mapenv.map_get(atoms::entry_slot())?.decode::<i64>()?)),
            "entry_prev_slot" => Global::new(&mut store, Value::I64(mapenv.map_get(atoms::entry_prev_slot())?.decode::<i64>()?)),
            "entry_height" => Global::new(&mut store, Value::I64(mapenv.map_get(atoms::entry_height())?.decode::<i64>()?)),
            "entry_epoch" => Global::new(&mut store, Value::I64(mapenv.map_get(atoms::entry_epoch())?.decode::<i64>()?)),
            "entry_vr_ptr" => Global::new(&mut store, Value::I32(10_300)),
            "entry_dr_ptr" => Global::new(&mut store, Value::I32(10_400)),

            "tx_signer_ptr" => Global::new(&mut store, Value::I32(11_000)),
            "tx_nonce" => Global::new(&mut store, Value::I64(mapenv.map_get(atoms::tx_nonce())?.decode::<i64>()?)),

            "account_current_ptr" => Global::new(&mut store, Value::I32(12_000)),
            "account_caller_ptr" => Global::new(&mut store, Value::I32(13_000)),
            "account_origin_ptr" => Global::new(&mut store, Value::I32(14_000)),

            "attached_symbol_ptr" => Global::new(&mut store, Value::I32(15_000)),
            "attached_amount_ptr" => Global::new(&mut store, Value::I32(16_000)),

            "import_attach" => Function::new_typed_with_env(&mut store, &host_env, import_attach_implementation),

            "import_log" => Function::new_typed_with_env(&mut store, &host_env, import_log_implementation),
            "import_return_value" => Function::new_typed_with_env(&mut store, &host_env, import_return_value_implementation),

            "import_call_0" => Function::new_typed_with_env(&mut store, &host_env, import_call_0_implementation),
            "import_call_1" => Function::new_typed_with_env(&mut store, &host_env, import_call_1_implementation),
            "import_call_2" => Function::new_typed_with_env(&mut store, &host_env, import_call_2_implementation),
            "import_call_3" => Function::new_typed_with_env(&mut store, &host_env, import_call_3_implementation),
            "import_call_4" => Function::new_typed_with_env(&mut store, &host_env, import_call_4_implementation),

            //storage
            "import_kv_put" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_put_implementation),
            "import_kv_increment" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_increment_implementation),
            "import_kv_delete" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_delete_implementation),
            "import_kv_clear" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_clear_implementation),

            "import_kv_get" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_get_implementation),
            "import_kv_exists" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_exists_implementation),
            "import_kv_get_prev" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_get_prev_implementation),
            "import_kv_get_next" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_get_next_implementation),

            //"import_kv_put_int" => Function::new_typed(&mut store, || println!("called_kv_put_in_rust")),
            //"import_kv_get_prefix" => Function::new_typed(&mut store, || println!("called_kv_get_in_rust")),
            //"import_kv_clear" => Function::new_typed(&mut store, || println!("called_kv_get_in_rust")),

            //AssemblyScript specific
            "abort" => Function::new_typed_with_env(&mut store, &host_env, abort_implementation),
            "seed" => Global::new(&mut store, Value::F64(mapenv.map_get(atoms::seedf64())?.decode::<f64>()?)),
        }
    };

    let instance = Instance::new(&mut store, &module, &import_object)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;

    host_env.as_mut(&mut store).instance = Some(Arc::new(instance.clone()));

    let instance_memory = instance.exports.get_memory("memory").map_err(|err| {
        rustler::Error::Term(Box::new(format!(
            "Failed to get 'memory' export from Wasm instance: {}",
            err
        )))
    })?;
    host_env.as_mut(&mut store).memory = Some(instance_memory.clone()); // Update env
    //let mut duration_since_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    //println!("ns3 {}", duration_since_epoch.as_nanos());

    let entry_to_call = instance
        .exports
        .get_function(&function_name)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let call_result = entry_to_call.call(&mut store, &wasm_args);
    //let mut duration_since_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    //println!("ns4 {}", duration_since_epoch.as_nanos());

    let remaining_u64 = match get_remaining_points(&mut store, &instance) {
        MeteringPoints::Remaining(value) => value,
        MeteringPoints::Exhausted => 0,
    };

    match call_result {
        Err(err) => {
            //println!("9 {:?}", get_remaining_points(&mut store, &instance));

            let data = host_env.as_ref(&store); // Read-only access to HostEnv data

            //println!("wtf {:?} {:?} {:?} {:?} {}", data.error, data.return_value, err.message(), err.to_string(), String::from_utf8_lossy(data.return_value.as_ref().unwrap()));

            let encoded_logs: Vec<Binary> = data
                .logs
                .iter()
                .map(|bytes| {
                    let mut bin = OwnedBinary::new(bytes.len()).unwrap();
                    bin.as_mut_slice().copy_from_slice(bytes);
                    Binary::from_owned(bin, env)
                })
                .collect();
            /*
                        let data_error = match &data.error {
                            Some(bytes) => {
                                let mut owned = OwnedBinary::new(bytes.len()).unwrap();
                                owned.as_mut_slice().copy_from_slice(&bytes);
                                Binary::from_owned(owned, env).encode(env)
                            },
                            None => { atoms::nil().encode(env) }
                        };
            */
            let return_value = match &data.return_value {
                Some(bytes) => {
                    let mut owned = OwnedBinary::new(bytes.len()).unwrap();
                    owned.as_mut_slice().copy_from_slice(&bytes);
                    Binary::from_owned(owned, env).encode(env)
                }
                None => atoms::nil().encode(env),
            };

            let payload = (
                atoms::result(),
                (err.message(), encoded_logs, remaining_u64, return_value),
            );
            env.send(&rpc_pid, payload.encode(env));

            //Ok((err.message(), data_error, encoded_logs, remaining_u64, return_value).encode(env))
            Ok(atoms::ok().encode(env))
        }
        Ok(_values) => {
            //println!("99 {:?}", get_remaining_points(&mut store, &instance));

            let data = host_env.as_ref(&store); // Read-only access to HostEnv data

            let encoded_logs: Vec<Binary> = data
                .logs
                .iter()
                .map(|bytes| {
                    let mut bin = OwnedBinary::new(bytes.len()).unwrap();
                    bin.as_mut_slice().copy_from_slice(bytes);
                    Binary::from_owned(bin, env)
                })
                .collect();

            let payload = (
                atoms::result(),
                (atoms::nil(), encoded_logs, remaining_u64, atoms::nil()),
            );
            env.send(&rpc_pid, payload.encode(env));

            //Ok((atoms::nil(), atoms::nil(), encoded_logs, remaining_u64, atoms::nil()).encode(env))
            Ok(atoms::ok().encode(env))
        }
    }
}

#[rustler::nif(schedule = "DirtyCpu")]
fn validate_contract<'a>(
    env: Env<'a>,
    mapenv: Term<'a>,
    wasm_bytes: Binary,
) -> Result<rustler::Term<'a>, rustler::Error> {
    let metering = Arc::new(Metering::new(10_000_000, cost_function));
    let mut compiler = Singlepass::default();
    compiler.canonicalize_nans(true);

    use wasmer::CompilerConfig;
    compiler.push_middleware(metering);

    let mut features = Features::new();
    //features.bulk_memory(false); #required for modern compilers to WASM
    features.threads(false);
    features.reference_types(false);
    features.simd(false);
    features.multi_value(false);
    features.tail_call(false);
    features.module_linking(false);
    features.multi_memory(false);
    features.memory64(false);

    let engine = EngineBuilder::new(compiler).set_features(Some(features));
    let mut store = Store::new(engine);

    let module = Module::new(&store, &wasm_bytes.as_slice())
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;

    let memory = Memory::new(&mut store, MemoryType::new(Pages(8), None, false))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;

    let it1 = mapenv
        .map_get(atoms::seed())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(10_000, &((32 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(10_004, it1)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it2 = mapenv
        .map_get(atoms::entry_signer())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(10_100, &((48 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(10_104, it2)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it3 = mapenv
        .map_get(atoms::entry_prev_hash())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(10_200, &((32 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(10_204, it3)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it4 = mapenv
        .map_get(atoms::entry_vr())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(10_300, &((96 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(10_304, it4)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it5 = mapenv
        .map_get(atoms::entry_dr())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(10_400, &((96 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(10_404, it5)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it6 = mapenv
        .map_get(atoms::tx_signer())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(11_000, &((48 as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(11_004, it6)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it7 = mapenv
        .map_get(atoms::account_current())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(12_000, &((it7.len() as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(12_004, it7)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it8 = mapenv
        .map_get(atoms::account_caller())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(13_000, &((it8.len() as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(13_004, it8)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it9 = mapenv
        .map_get(atoms::account_origin())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(14_000, &((it9.len() as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(14_004, it9)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it10 = mapenv
        .map_get(atoms::attached_symbol())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(15_000, &((it10.len() as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(15_004, it10)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    let it11 = mapenv
        .map_get(atoms::attached_amount())?
        .decode::<Binary>()?
        .as_slice();
    memory
        .view(&mut store)
        .write(16_000, &((it11.len() as i32).to_le_bytes()))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;
    memory
        .view(&mut store)
        .write(16_004, it11)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;

    let host_env = FunctionEnv::new(
        &mut store,
        HostEnv {
            memory: None,
            error: None,
            return_value: None,
            logs: vec![],
            readonly: true,
            rpc_pid: env.pid(),
            current_account: it7.to_vec(),
            instance: None,
            attached_symbol: Vec::new(),
            attached_amount: Vec::new(),
        },
    );

    let import_object = imports! {
        "env" => {
            "memory" => memory,
            "seed_ptr" => Global::new(&mut store, Value::I32(10_000)),
            "entry_signer_ptr" => Global::new(&mut store, Value::I32(10_100)),
            "entry_prev_hash_ptr" => Global::new(&mut store, Value::I32(10_200)),
            "entry_slot" => Global::new(&mut store, Value::I64(mapenv.map_get(atoms::entry_slot())?.decode::<i64>()?)),
            "entry_prev_slot" => Global::new(&mut store, Value::I64(mapenv.map_get(atoms::entry_prev_slot())?.decode::<i64>()?)),
            "entry_height" => Global::new(&mut store, Value::I64(mapenv.map_get(atoms::entry_height())?.decode::<i64>()?)),
            "entry_epoch" => Global::new(&mut store, Value::I64(mapenv.map_get(atoms::entry_epoch())?.decode::<i64>()?)),
            "entry_vr_ptr" => Global::new(&mut store, Value::I32(10_300)),
            "entry_dr_ptr" => Global::new(&mut store, Value::I32(10_400)),

            "tx_signer_ptr" => Global::new(&mut store, Value::I32(11_000)),
            "tx_nonce" => Global::new(&mut store, Value::I64(mapenv.map_get(atoms::tx_nonce())?.decode::<i64>()?)),

            "account_current_ptr" => Global::new(&mut store, Value::I32(12_000)),
            "account_caller_ptr" => Global::new(&mut store, Value::I32(13_000)),
            "account_origin_ptr" => Global::new(&mut store, Value::I32(14_000)),

            "attached_symbol_ptr" => Global::new(&mut store, Value::I32(15_000)),
            "attached_amount_ptr" => Global::new(&mut store, Value::I32(16_000)),

            "import_attach" => Function::new_typed_with_env(&mut store, &host_env, import_attach_implementation),

            "import_log" => Function::new_typed_with_env(&mut store, &host_env, import_log_implementation),
            "import_return_value" => Function::new_typed_with_env(&mut store, &host_env, import_return_value_implementation),

            "import_call_0" => Function::new_typed_with_env(&mut store, &host_env, import_call_0_implementation),
            "import_call_1" => Function::new_typed_with_env(&mut store, &host_env, import_call_1_implementation),
            "import_call_2" => Function::new_typed_with_env(&mut store, &host_env, import_call_2_implementation),
            "import_call_3" => Function::new_typed_with_env(&mut store, &host_env, import_call_3_implementation),
            "import_call_4" => Function::new_typed_with_env(&mut store, &host_env, import_call_4_implementation),

            //storage
            "import_kv_put" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_put_implementation),
            "import_kv_increment" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_increment_implementation),
            "import_kv_delete" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_delete_implementation),
            "import_kv_clear" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_clear_implementation),

            "import_kv_get" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_get_implementation),
            "import_kv_exists" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_exists_implementation),
            "import_kv_get_prev" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_get_prev_implementation),
            "import_kv_get_next" => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_get_next_implementation),

            //"import_kv_put_int" => Function::new_typed(&mut store, || println!("called_kv_put_in_rust")),
            //"import_kv_get_prefix" => Function::new_typed(&mut store, || println!("called_kv_get_in_rust")),

            //AssemblyScript specific
            "abort" => Function::new_typed_with_env(&mut store, &host_env, abort_implementation),
            "seed" => Global::new(&mut store, Value::F64(mapenv.map_get(atoms::seedf64())?.decode::<f64>()?)),
        }
    };

    let instance = Instance::new(&mut store, &module, &import_object)
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;

    Ok(atoms::ok().encode(env))
}

rustler::init!("Elixir.WasmerEx");
