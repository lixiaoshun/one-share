use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("mDNS error: {0}")]
    MDNSError(#[from] mdns_sd::Error),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub name: String,
    pub ip: IpAddr,
    pub port: u16,
}

pub struct Discovery {
    mdns: ServiceDaemon,
    peers: Arc<Mutex<HashMap<String, PeerInfo>>>,
    tx: mpsc::Sender<PeerInfo>,
    service_name: String,
}

impl Discovery {
    pub fn new() -> Result<(Self, mpsc::Receiver<PeerInfo>), DiscoveryError> {
        let mdns = ServiceDaemon::new()?;
        let peers = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = mpsc::channel(100);
        let service_name = hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let port = 8000; // 使用固定端口或从配置中获取
        let service_info = mdns_sd::ServiceInfo::new(
            "_oneshare._tcp.local.",
            &service_name,
            &service_name,
            "0.0.0.0",
            port,
            &[("version", "1.0")],
        )?;
        mdns.register(service_info)?;

        let discovery = Self {
            mdns,
            peers: peers.clone(),
            tx: tx.clone(),
            service_name,
        };

        // 浏览网络中的其他服务
        let browse_receiver = discovery.mdns.browse("_oneshare._tcp.local.")?;
        let peers_clone = peers.clone();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            while let Ok(event) = browse_receiver.recv_async().await {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        if let Some(ip) = info.get_addresses().iter().next() {
                            let peer = PeerInfo {
                                id: info.get_fullname().to_string(),
                                name: info.get_hostname().to_string(),
                                ip: *ip,
                                port: info.get_port(),
                            };
                            peers_clone
                                .lock()
                                .unwrap()
                                .insert(peer.id.clone(), peer.clone());
                            let _ = tx_clone.send(peer).await;
                        }
                    }
                    ServiceEvent::ServiceRemoved(name, _type) => {
                        peers_clone.lock().unwrap().remove(&name);
                    }
                    _ => {}
                }
            }
        });

        Ok((discovery, rx))
    }

    pub fn get_peers(&self) -> Vec<PeerInfo> {
        self.peers.lock().unwrap().values().cloned().collect()
    }
}
