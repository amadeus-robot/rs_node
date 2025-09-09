use rand::Rng;
use std::net::IpAddr;
use tokio::task;

use crate::{NodePeers, node_proto::NodeProto};

pub enum BroadcastKind {
    TxPool,
    Entry,
    AttestationBulk,
    Sol,
    SpecialBusiness,
}

pub struct NodeGen {}

impl NodeGen {
    pub fn init() {
        
    }

    pub fn get_socket_gen() -> String {
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..8); // 0..7
        format!("NodeGenSocketGen{}", idx)
    }
    /// Async broadcast function
    pub fn broadcast(
        kind: BroadcastKind,
        who: &str,
        payload: Vec<u8>,
        socket_sender: tokio::sync::mpsc::Sender<(Vec<IpAddr>, Vec<u8>)>,
    ) {
        let socket_sender = socket_sender.clone();
        let who = who.to_string();

        task::spawn(async move {
            let msg = match kind {
                BroadcastKind::TxPool => NodeProto::txpool(&payload),
                BroadcastKind::Entry => NodeProto::entry(&payload),
                BroadcastKind::AttestationBulk => NodeProto::attestation_bulk(&payload),
                BroadcastKind::Sol => NodeProto::sol(&payload),
                BroadcastKind::SpecialBusiness => NodeProto::special_business(&payload),
            };
            let ips = NodePeers::by_who(&who);
            let compressed = NodeProto::compress(&msg);
            let _ = socket_sender.send((ips, compressed)).await;
        });
    }
}
