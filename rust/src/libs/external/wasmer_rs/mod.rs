use std::sync::Arc;

use crate::MapEnv;
use wasmer::wasmparser::Operator;
use wasmer::{CompilerConfig, FunctionEnv, Instance, Value, imports};
use wasmer::{Memory, MemoryType, Module, Pages, Store};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_middlewares::Metering;

pub struct WasmerRs;

impl WasmerRs {
    fn write_to_memory(
        store: &mut Store,
        memory: &Memory,
        base_offset: u64,
        data: &[u8],
    ) -> Result<(), String> {
        let length = data.len();

        memory
            .view(store)
            .write(base_offset, &(length as i32).to_le_bytes())
            .map_err(|err| err.to_string())?;

        memory
            .view(store)
            .write(base_offset + 4, data)
            .map_err(|err| err.to_string())?;

        Ok(())
    }

    fn cost_function(operator: &Operator) -> u64 {
        10
    }
    pub fn call(
        mapenv: MapEnv,
        wasmbytes: Vec<u8>,
        function: String,
        args: Vec<String>,
    ) -> Result<ExecutionResult, anyhow::Error> {
        // let cost_function = Self::cost_function();
        let metering = Arc::new(Metering::new(
            mapenv.call_exec_points_remaining,
            cost_function,
        ));

        // Configure compiler
        let mut compiler = Singlepass::default();
        compiler.canonicalize_nans(true);
        compiler.push_middleware(metering.clone());

        // Set features
        let mut features = wasmer::Features::new();
        features.threads(false);
        features.reference_types(false);
        features.simd(false);
        features.multi_value(false);
        features.tail_call(false);
        features.module_linking(false);
        features.multi_memory(false);
        features.memory64(false);

        // Create engine and store
        let engine = wasmer::EngineBuilder::new(compiler)
            .set_features(Some(features))
            .engine();
        let mut store = Store::new(engine);

        // Compile module
        let module = Module::new(&store, wasmbytes)
            .map_err(|err| err.to_string())
            .unwrap();

        // Create memory
        let memory = Memory::new(&mut store, MemoryType::new(Pages(8), None, false))
            .map_err(|err| err.to_string())
            .unwrap();

        // Write context data to memory
        Self::write_to_memory(&mut store, &memory, 10_000, &mapenv.seed.unwrap());
        Self::write_to_memory(&mut store, &memory, 10_100, &mapenv.entry_signer);
        Self::write_to_memory(&mut store, &memory, 10_200, &mapenv.entry_prev_hash);
        Self::write_to_memory(&mut store, &memory, 10_300, &mapenv.entry_vr);
        Self::write_to_memory(&mut store, &memory, 10_400, &mapenv.entry_dr);
        Self::write_to_memory(&mut store, &memory, 11_000, &mapenv.tx_signer.unwrap());
        Self::write_to_memory(
            &mut store,
            &memory,
            12_000,
            &mapenv.account_current.unwrap(),
        );
        Self::write_to_memory(&mut store, &memory, 13_000, &mapenv.account_caller.unwrap());
        Self::write_to_memory(&mut store, &memory, 14_000, &mapenv.account_origin.unwrap());
        Self::write_to_memory(
            &mut store,
            &memory,
            15_000,
            &mapenv.attached_symbol.as_bytes(),
        )
        .unwrap();
        Self::write_to_memory(
            &mut store,
            &memory,
            16_000,
            &mapenv.attached_amount.to_le_bytes(),
        );

        // Create host environment
        let host_env = FunctionEnv::new(
            &mut store,
            HostEnv {
                memory: None,
                error: None,
                return_value: None,
                logs: vec![],
                readonly: mapenv.readonly,
                current_account: mapenv.account_current.clone(),
                attached_symbol: mapenv.attached_symbol.clone(),
                attached_amount: mapenv.attached_amount.clone(),
            },
        );

        // Create import object
        let import_object = imports! {
            "env" => {
                "memory" => memory.clone(),
                "seed_ptr" => wasmer::Global::new(&mut store, Value::I32(10_000)),
                "entry_signer_ptr" => wasmer::Global::new(&mut store, Value::I32(10_100)),
                "entry_prev_hash_ptr" => wasmer::Global::new(&mut store, Value::I32(10_200)),
                "entry_slot" => wasmer::Global::new(&mut store, Value::I64(mapenv.entry_slot as i64)),
                "entry_prev_slot" => wasmer::Global::new(&mut store, Value::I64(mapenv.entry_prev_slot as i64)),
                "entry_height" => wasmer::Global::new(&mut store, Value::I64(mapenv.entry_height as i64)),
                "entry_epoch" => wasmer::Global::new(&mut store, Value::I64(mapenv.entry_epoch as i64)),
                "entry_vr_ptr" => wasmer::Global::new(&mut store, Value::I32(10_300)),
                "entry_dr_ptr" => wasmer::Global::new(&mut store, Value::I32(10_400)),
                "tx_signer_ptr" => wasmer::Global::new(&mut store, Value::I32(11_000)),
                "tx_nonce" => wasmer::Global::new(&mut store, Value::I64(mapenv.tx_nonce.unwrap() as i64)),
                "account_current_ptr" => wasmer::Global::new(&mut store, Value::I32(12_000)),
                "account_caller_ptr" => wasmer::Global::new(&mut store, Value::I32(13_000)),
                "account_origin_ptr" => wasmer::Global::new(&mut store, Value::I32(14_000)),
                "attached_symbol_ptr" => wasmer::Global::new(&mut store, Value::I32(15_000)),
                "attached_amount_ptr" => wasmer::Global::new(&mut store, Value::I32(16_000)),
            }
        };

        // Create instance
        let instance = Instance::new(&mut store, &module, &import_object)
            .map_err(|err| err.to_string())
            .unwrap();

        // Update host environment with instance memory
        let instance_memory = instance
            .exports
            .get_memory("memory")
            .map_err(|err| err.to_string())
            .unwrap();
        host_env.as_mut(&mut store).memory = Some(instance_memory.clone());

        // Get function and call it
        let function = instance
            .exports
            .get_function(&function)
            .map_err(|err| err.to_string())
            .unwrap();

        let call_result = function.call(&mut store, &args);

        // Get remaining points
        let remaining_points = match metering.get_remaining_points(&store, &instance) {
            Some(points) => points,
            None => 0,
        };

        // Read results from host environment
        let env_data = host_env.as_ref(&store);

        Ok(ExecutionResult {
            success: call_result.is_ok(),
            error: env_data.error.clone(),
            logs: env_data.logs.clone(),
            return_value: env_data.return_value.clone(),
            remaining_points,
        })
    }
}

pub struct ExecutionResult {
    pub success: bool,
    pub error: Option<Vec<u8>>,
    pub logs: Vec<Vec<u8>>,
    pub return_value: Option<Vec<u8>>,
    pub remaining_points: u64,
}
