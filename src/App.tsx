import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface Peer {
  id: string;
  name: string;
  ip: string;
  port: number;
}

function App() {
  const [peers, setPeers] = useState<Peer[]>([]);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [isSharing, setIsSharing] = useState(false);
  const [transferProgress, setTransferProgress] = useState(0);
  const [discoveryStatus, setDiscoveryStatus] = useState<
    "idle" | "discovering" | "error"
  >("idle");

  useEffect(() => {
    // 启动设备发现服务
    startDiscovery();
    // 定期更新在线设备列表
    const interval = setInterval(updatePeers, 5000);
    return () => clearInterval(interval);
  }, []);

  async function startDiscovery() {
    try {
      setDiscoveryStatus("discovering");
      await invoke("start_discovery");
      await updatePeers(); // 立即尝试获取设备列表
    } catch (error) {
      console.error("Failed to start discovery:", error);
      setDiscoveryStatus("error");
    }
  }

  async function updatePeers() {
    if (discoveryStatus !== "discovering") return;

    try {
      const peerList = await invoke<Peer[]>("get_peers");
      setPeers(peerList);
    } catch (error) {
      console.error("Failed to get peers:", error);
      setDiscoveryStatus("error");
    }
  }

  async function handleFileSelect(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (file) {
      setSelectedFile(file);
    }
  }

  async function sendFileToPeer(peer: Peer) {
    if (!selectedFile) return;
    try {
      await invoke("send_file", {
        path: selectedFile.path,
        peer: peer,
      });
    } catch (error) {
      console.error("Failed to send file:", error);
    }
  }

  async function toggleScreenShare() {
    try {
      if (!isSharing) {
        await invoke("start_screen_share");
        setIsSharing(true);
      } else {
        await invoke("stop_screen_share");
        setIsSharing(false);
      }
    } catch (error) {
      console.error("Screen share error:", error);
      setIsSharing(false);
    }
  }

  return (
    <div className="app-container">
      <header className="app-header">
        <h1>OneShare</h1>
      </header>

      <main className="main-content">
        <section className="peers-section">
          <h2>在线设备</h2>
          {discoveryStatus === "error" && (
            <div className="error-message">设备发现服务出错，请重试</div>
          )}
          {discoveryStatus === "discovering" && peers.length === 0 && (
            <div className="info-message">正在搜索局域网内的设备...</div>
          )}
          <div className="peers-list">
            {peers.map((peer) => (
              <div key={peer.id} className="peer-item">
                <span>{peer.name}</span>
                <button
                  onClick={() => sendFileToPeer(peer)}
                  disabled={!selectedFile}
                >
                  发送文件
                </button>
              </div>
            ))}
          </div>
        </section>

        <section className="file-section">
          <h2>文件传输</h2>
          <input
            type="file"
            onChange={handleFileSelect}
            className="file-input"
          />
          {transferProgress > 0 && (
            <div className="progress-bar">
              <div
                className="progress"
                style={{ width: `${transferProgress}%` }}
              ></div>
            </div>
          )}
        </section>

        <section className="screen-share-section">
          <h2>屏幕共享</h2>
          <button
            onClick={toggleScreenShare}
            className={`share-button ${isSharing ? "sharing" : ""}`}
          >
            {isSharing ? "停止共享" : "开始共享"}
          </button>
        </section>
      </main>
    </div>
  );
}

export default App;
