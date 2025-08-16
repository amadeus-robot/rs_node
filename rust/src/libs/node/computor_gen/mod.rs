use std::sync::{Arc, Mutex, mpsc};

pub mod computor_gen_type;
use chrono::Utc;
pub use computor_gen_type::*;

// pub struct ComputorGen {
//     pub state: Arc<Mutex<ComputorState>>,
//     pub tx: mpsc::Sender<ComputorMsg>,
// }

// impl ComputorGen {
//     fn new(tx: mpsc::Sender<ComputorMsg>, state: Arc<Mutex<ComputorState>>) -> Self {
//         Self { state, tx }
//     }

//     async fn handle_message(&self, msg: ComputorMsg) {
//         match msg {
//             ComputorMsg::Tick => {
//                 let mut state = self.state.lock().await;
//                 if !state.enabled {
//                     return;
//                 }
//                 if !fabric_sync_attest_gen::is_quorum_in_epoch() {
//                     println!("🔴 cannot compute: out_of_sync");
//                 } else {
//                     self.tick(&mut state).await;
//                 }
//             }
//             ComputorMsg::Start(ctype) => {
//                 let mut state = self.state.lock().await;
//                 state.enabled = true;
//                 state.ctype = ctype;
//             }
//             ComputorMsg::Stop => {
//                 let mut state = self.state.lock().await;
//                 state.enabled = false;
//             }
//             ComputorMsg::SetEmissionAddress(to_addr) => {
//                 let sk = "trainer_sk_from_config";
//                 let tx_bin = tx::build(sk, "Epoch", "set_emission_address", vec![to_addr]);
//                 txpool::insert(&tx_bin);
//                 node_gen::broadcast("txpool", "trainers", vec![tx_bin]);
//             }
//         }
//     }

//     async fn tick(&self, state: &mut ComputorState) {
//         println!("computor running {}", Utc::now());
//         let pk = "trainer_pk_from_config";
//         let pop = "trainer_pop_from_config";

//         let coins = consensus::chain_balance(pk);
//         let epoch = consensus::chain_epoch();
//         let has_exec_coins = coins >= bic_coin::to_cents(100);

//         let sol = if (state.ctype.as_deref() == Some("trainer") && !has_exec_coins)
//             || state.ctype.is_none()
//         {
//             upow::compute_for(
//                 epoch,
//                 "entry_signer",
//                 "entry_pop",
//                 pk,
//                 rand::random::<[u8; 96]>().to_vec(),
//                 100,
//             )
//         } else {
//             upow::compute_for(epoch, pk, pop, pk, rand::random::<[u8; 96]>().to_vec(), 100)
//         };

//         if let Some(sol) = sol {
//             if state.ctype.as_deref() == Some("trainer") && !has_exec_coins {
//                 println!("🔢 tensor matmul complete! broadcasting sol..");
//                 node_gen::broadcast("sol", "trainers", vec![sol]);
//             } else {
//                 let sk = "trainer_sk_from_config";
//                 let tx_bin = tx::build(sk, "Epoch", "submit_sol", vec![sol]);
//                 let hash = tx::unpack(&tx_bin);
//                 println!("🔢 tensor matmul complete! tx {}", hash);
//                 txpool::insert(&tx_bin);
//                 node_gen::broadcast("txpool", "trainers", vec![tx_bin]);
//             }
//         }
//     }
// }
