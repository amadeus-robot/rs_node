use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Consensus, EntryHeader, Fabric, NodeMsg, CONFIG};

#[derive(Debug, Clone)]
pub struct Tip {
    pub signature: Vec<u8>,
    pub header_unpacked: EntryHeader,
    pub mask: Option<Vec<u8>>, // optional mask
}

pub struct NodeProto {}

impl NodeProto {
    pub fn ping() -> NodeMsg {
        let tip = Consensus::chain_tip_entry();
        let temporal = Tip {
            header_unpacked: tip.header_unpacked,
            mask: tip.mask,
            signature: tip.signature,
        };

        let fabric_temporal = Fabric::rooted_tip_entry().unwrap();
        let rooted = Tip {
            header_unpacked: fabric_temporal.header_unpacked,
            mask: fabric_temporal.mask,
            signature: fabric_temporal.signature,
        };

        let ts_m = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        NodeMsg::Ping {
            op: "ping".to_string(),
            temporal, // must be Vec<u8>
            rooted,   // must be Vec<u8>
            ts_m,
        }
    }

    pub fn pong(ts_m: u128) -> NodeMsg {
        NodeMsg::Pong {
            op: "pong".to_string(),
            ts_m,
        }
    }

    pub fn new_phone_who_dis() -> NodeMsg {
        let anr = CONFIG.ama
        NodeMsg::NewPhoneWhoDis {
            op: "new_phone_who_dis".to_string(),
            anr: (),
            challenge: (),
        }
    }

    pub fn txpool(param: &[u8]) -> Vec<u8> {
        vec![]
    }
    pub fn entry(param: &[u8]) -> Vec<u8> {
        vec![]
    }
    pub fn attestation_bulk(param: &[u8]) -> Vec<u8> {
        vec![]
    }
    pub fn sol(param: &[u8]) -> Vec<u8> {
        vec![]
    }
    pub fn special_business(param: &[u8]) -> Vec<u8> {
        vec![]
    }
    pub fn compress(param: &[u8]) -> Vec<u8> {
        vec![]
    }
}
