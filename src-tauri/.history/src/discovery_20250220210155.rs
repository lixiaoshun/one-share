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
        let hostname = hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let service_name = format!("{}._oneshare._tcp.local.", hostname);

        Ok((
            Self {
                mdns,
                peers,
                tx,
                service_name,
            },
            rx,
        ))
    }

    pub fn start_discovery(&self) -> Result<(), DiscoveryError> {
        // 注册本地服务
        let port = 8000; // 使用固定端口或动态分配
        let service_info = mdns_sd::ServiceInfo::new(
            "_oneshare._tcp.local.",
            &self.service_name,
            "localhost",
            "",
            port,
            &["version=1.0"],
        )?;
        self.mdns.register(service_info)?;

        // 浏览网络中的其他服务
        let browse_receiver = self.mdns.browse("_oneshare._tcp.local.")?;
        let peers = self.peers.clone();
        let tx = self.tx.clone();

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
                            peers.lock().unwrap().insert(peer.id.clone(), peer.clone());
                            let _ = tx.send(peer);
                        }
                    }
                    ServiceEvent::ServiceRemoved(name, _type) => {
                        peers.lock().unwrap().remove(&name);
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub fn get_peers(&self) -> Vec<PeerInfo> {
        self.peers.lock().unwrap().values().cloned().collect()
    }
}
