use std::net::{IpAddr, SocketAddr};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task;
use std::sync::Arc;

#[derive(Clone)]
pub struct NodeGenSocketGen {
    name: String,
    ip: IpAddr,
    port: u16,
    socket: Arc<UdpSocket>,
    tx: UnboundedSender<NodeMessage>,
}

#[derive(Debug)]
pub enum NodeMessage {
    UdpReceived(SocketAddr, Vec<u8>),
    SendToSome(Vec<String>, Vec<u8>),
}

impl NodeGenSocketGen {
    pub async fn start(ip: IpAddr, port: u16, name: Option<String>) -> anyhow::Result<()> {
        let name = name.unwrap_or_else(|| "NodeGenSocketGen".to_string());
        let socket = UdpSocket::bind(SocketAddr::new(ip, port)).await?;
        let socket = Arc::new(socket);

        // Spawn task for receiving UDP packets
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let recv_socket = socket.clone();
        task::spawn(async move {
            let mut buf = vec![0u8; 65536];
            loop {
                match recv_socket.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        let data = buf[..len].to_vec();
                        let _ = tx.send(NodeMessage::UdpReceived(addr, data));
                    }
                    Err(e) => eprintln!("UDP recv error: {:?}", e),
                }
            }
        });

        let state = NodeGenSocketGen {
            name,
            ip,
            port,
            socket: socket.clone(),
            tx: tx.clone(),
        };

        // Spawn main message handler loop
        task::spawn(state.run(rx));

        Ok(())
    }

    async fn run(mut self, mut rx: UnboundedReceiver<NodeMessage>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                NodeMessage::UdpReceived(addr, data) => {
                    let tx_clone = self.tx.clone();
                    task::spawn(async move {
                        if let Ok(parsed) = NodeProto::unpack_message_v2(&data) {
                            match parsed {
                                NodeProtoMessage::SignatureV1 { pk, signature, payload, shard_total, version } => {
                                    let valid = BlsRs::verify(&pk, &signature, &NodeProto::hash_payload(&pk, &payload));
                                    if valid && NodeANR::handshaked_and_valid_ip4(&pk, &addr.ip().to_string()) {
                                        let msg = NodeProto::deflate_decompress(&payload);
                                        NodeState::handle(msg.op, Peer { ip: addr.ip().to_string(), signer: pk, version }, msg);
                                    }
                                }
                                NodeProtoMessage::EncryptedShard { pk, ts_nano, shard_index, shard_total, payload, version } => {
                                    if NodeANR::handshaked_and_valid_ip4(&pk, &addr.ip().to_string()) {
                                        let shared_secret = NodePeers::get_shared_secret(&pk);
                                        let gen = NodeGen::get_reassembly_gen(&pk, ts_nano);
                                        gen.send(ReassemblyMessage::AddShard {
                                            pk,
                                            ts_nano,
                                            shard_index,
                                            shard_total,
                                            payload,
                                            version,
                                            shared_secret,
                                            peer_ip: addr.ip().to_string(),
                                        }).unwrap();
                                    }
                                }
                                _ => {}
                            }
                        }
                    });
                }

                NodeMessage::SendToSome(peer_ips, msg_compressed) => {
                    let port = crate::config::UDP_PORT;
                    for ip_str in peer_ips {
                        if let Ok(peer) = NodePeers::by_ip(&ip_str) {
                            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                                let msgs_packed = NodeProto::encrypt_message_v2(&msg_compressed, &peer.shared_secret);
                                for msg_packed in msgs_packed {
                                    let socket = self.socket.clone();
                                    let addr = SocketAddr::new(ip, port);
                                    task::spawn(async move {
                                        let _ = socket.send_to(&msg_packed, addr).await;
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
