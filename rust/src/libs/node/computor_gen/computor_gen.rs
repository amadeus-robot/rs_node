use crate::*;
use bs58::encode;
use futures_util::lock::Mutex;
use rand::RngCore;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::{interval, sleep};

#[derive(Debug)]
pub enum ComputorMessage {
    Start(Option<ComputorType>),
    Stop,
    Tick,
}
pub struct ComputorGen {
    state: Arc<Mutex<ComputorState>>,
    sender: UnboundedSender<ComputorMessage>,
    receiver: UnboundedReceiver<ComputorMessage>,
}

impl ComputorGen {
    /// Create new ComputorGen and return it
    pub fn start_link() -> Self {
        let (sender, receiver) = unbounded_channel();
        let state = Arc::new(Mutex::new(ComputorState {
            enabled: false,
            ctype: Some(ComputorType::None),
        }));

        ComputorGen {
            state,
            sender,
            receiver,
        }
    }

    /// Access sender to send messages
    pub fn sender(&self) -> UnboundedSender<ComputorMessage> {
        self.sender.clone()
    }

    /// Main async loop
    pub async fn run(&mut self) {
        let state = self.state.clone();
        let sender = self.sender.clone();

        // Spawn recurring tick task
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let _ = sender.send(ComputorMessage::Tick);
            }
        });

        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ComputorMessage::Start(t) => {
                    let mut st = state.lock().await;
                    st.enabled = true;
                    st.ctype = t;
                }
                ComputorMessage::Stop => {
                    let mut st = state.lock().await;
                    st.enabled = false;
                }
                ComputorMessage::Tick => {
                    self.handle_tick().await;
                }
            }
        }
    }

    async fn handle_tick(&self) {
        let st = self.state.lock().await;
        if !st.enabled {
            return;
        }
        drop(st); // unlock before async calls

        if !FabricSyncAttestGen::is_quorum_in_epoch() {
            println!("🔴 cannot compute: out_of_sync");
            return;
        }

        self.tick().await;
    }

    async fn tick(&self) {
        println!("computor running {:?}", chrono::Utc::now());

        let pk = AMACONFIG.trainer_pk();
        let pop = AMACONFIG.trainer_pop();

        let coins = Consensus::chain_balance(&pk, None);
        let epoch = Consensus::chain_epoch();
        let has_exec_coins = coins >= Coin::to_cents(100) as u64;

        let st = self.state.lock().await;
        let is_trainer = matches!(st.ctype, Some(ComputorType::Trainer));
        drop(st);

        // Generate random bytes
        let mut rand_bytes = [0u8; 96];
        rand::thread_rng().fill_bytes(&mut rand_bytes);

        if (is_trainer && !has_exec_coins) || self.state.lock().await.ctype.is_none() {
            if let Some(sol) = UPOW::compute_for(
                epoch,
                &EntryGenesis::signer(),
                &EntryGenesis::pop(),
                &pk,
                &rand_bytes,
                100,
            ) {
                println!("🔢 tensor matmul complete! broadcasting sol..");
                NodeGen::broadcast(BroadcastKind::Sol, "trainers", sol, self.sender.clone());
            }
        } else {
            if let Some(sol) = UPOW::compute_for(epoch, &pk, &pop, &pk, &rand_bytes, 100) {
                let sk = AMACONFIG.trainer_sk;
                let packed_tx = TX::build(&sk, "Epoch", "submit_sol", &[sol.clone()]);
                let hash = TX::unpack(&packed_tx).hash;
                println!("🔢 tensor matmul complete! tx {:?}", base58::encode(hash));

                TXPool::insert(packed_tx.clone());
                NodeGen::broadcast(
                    BroadcastKind::TxPool,
                    "trainers",
                    packed_tx,
                    socket_sender.clone(),
                );
            }
        }
    }

    pub async fn set_emission_address(to_address: &str) {
        // let sk = AMACONFIG.trainer_sk;
        // let packed_tx = TX::build(
        //     &sk,
        //     "Epoch",
        //     "set_emission_address",
        //     &[to_address.to_string()],
        // );
        // TXPool::insert(packed_tx.clone());
        // NodeGen::broadcast("txpool", "trainers", &[vec![packed_tx]]);
    }
}
