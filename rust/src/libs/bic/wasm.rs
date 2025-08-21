use std::{
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use rocksdb::Env;
use tokio::{
    sync::mpsc,
    time::{sleep, timeout},
};

use crate::{ConsensusKV, MapEnv, WASM_SAFE};

#[derive(Debug)]
pub enum Message {
    RustRequestStorageKvGet {
        rpc_id: u64,
        key: String,
    },
    RustRequestStorageKvPut {
        rpc_id: u64,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    RustRequestCall {
        rpc_id: u64,
        exec_remaining: u64,
        contract: String,
        function: String,
        args: Vec<Vec<u8>>,
        attached_symbol: Option<String>,
        attached_amount: Option<u64>,
    },
    Result {
        error: Option<String>,
        logs: Vec<String>,
        exec_remaining: u64,
        retv: Option<Vec<u8>>,
    },
    Timeout,
}

pub struct CallFrame {
    pub pid: u64, // placeholder for spawned WASM
    pub rpc_id: u64,
    pub last_account: String,
    pub last_caller: String,
}

pub struct WASM;

impl WASM {
    pub fn call(
        mapenv: MapEnv,
        wasmbytes: &[u8],
        function: &str,
        args: &[String],
    ) -> Result<Vec<u8>, String> {
        WASM_SAFE::spawn(mapenv, wasmbytes, function, args);
        // Placeholder for actual WASM call logic
        // This should interact with the WASM runtime to execute the function
        Ok(vec![]) // Return empty vector as placeholder
    }

    pub async fn wasm_loop(
        mut env: MapEnv,
        mut callstack: Vec<CallFrame>,
        rx: &mut Receiver<Message>,
        tx: Sender<Message>,
    ) -> MapEnv {
        let (tx, mut rx) = mpsc::channel::<Message>(100);

        loop {
            let msg = timeout(Duration::from_millis(1_000), rx.recv()).await;

            match msg {
                Ok(Some(msg)) => match msg {
                    Message::RustRequestStorageKvGet { rpc_id, key } => {
                        let value = ConsensusKV::kv_get(&key.as_bytes());
                        // respond to WASM runtime
                        continue;
                    }
                    Message::RustRequestStorageKvPut { rpc_id, key, value } => {
                        ConsensusKV::kv_put(key, value);
                        // respond to WASM runtime
                        continue;
                    }
                    Message::RustRequestCall {
                        rpc_id,
                        exec_remaining,
                        contract,
                        function,
                        args,
                        attached_symbol,
                        attached_amount,
                    } => {
                        // preserve your Elixir nested logic for attached_amount, call_counter, bytecode, seed, etc.
                        continue;
                    }
                    Message::Result {
                        error,
                        logs,
                        exec_remaining,
                        retv,
                    } => {
                        // preserve your Elixir result handling logic
                        continue;
                    }
                    Message::Timeout => {
                        // terminate any spawned instances
                        break;
                    }
                },
                _ => break,
            }
        }

        env
    }
}
