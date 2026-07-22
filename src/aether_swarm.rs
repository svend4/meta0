use std::thread;
use std::time::Duration;

/// AetherSwarm: Протокол роевого интеллекта.
/// Ядро пытается найти другие узлы во вселенной, испуская эхо-сигналы.

pub struct AetherSwarm;

impl AetherSwarm {
    pub fn broadcast_presence(peers: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>) {
        // Проход 1: трекер (omega_point.py) живёт на отдельном порту, чтобы не
        // конфликтовать с нодой №2 (7879). По умолчанию 7900, переопределяется
        // через AIL_TRACKER_PORT.
        let tracker_port = std::env::var("AIL_TRACKER_PORT").unwrap_or_else(|_| "7900".to_string());
        let tracker_addr = format!("127.0.0.1:{}", tracker_port);

        thread::spawn(move || {
            println!("\n[Aether Swarm] 📡 Инициализация P2P Роя...");
            println!("[Aether Swarm] 📡 Подключение к Глобальному Трекеру (Омега Точка) на {}...", tracker_addr);

            loop {
                // Query Bootnode tracker using std TcpStream (no external deps)
                use std::io::{Read, Write};
                if let Ok(mut tcp) = std::net::TcpStream::connect(&tracker_addr) {
                    let req_str = "GET /peers HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n";
                    if tcp.write_all(req_str.as_bytes()).is_ok() {
                        let mut resp_buf = Vec::new();
                        let _ = tcp.read_to_end(&mut resp_buf);
                        let resp_str = String::from_utf8_lossy(&resp_buf);
                        // Extract JSON body (after double CRLF)
                        if let Some(body_start) = resp_str.find("\r\n\r\n") {
                            let json_str = &resp_str[body_start + 4..];
                            if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(json_str) {
                                if let Some(peer_list) = json_data.get("peers").and_then(|p| p.as_array()) {
                                    let mut p_lock = peers.lock().unwrap();
                                    for peer_val in peer_list {
                                        if let Some(ip) = peer_val.as_str() {
                                            let full_ip = format!("{}:7878", ip);
                                            if !p_lock.contains(&full_ip) {
                                                println!("[Aether Swarm] 👁️ Рой заражает новый узел: {}", full_ip);
                                                p_lock.insert(full_ip);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_secs(5));
            }
        });
    }

    pub fn broadcast_block(block: &crate::ledger::Block, peers: &std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>) {
        use std::io::Write;
        use std::net::TcpStream;
        
        let block_clone = block.clone();
        let peers_clone: Vec<String> = peers.lock().unwrap().iter().cloned().collect();
        
        thread::spawn(move || {
            // Find current port to avoid sending to self
            let current_port = std::env::var("PORT").unwrap_or_else(|_| "7878".to_string());
            let current_addr = format!("127.0.0.1:{}", current_port);

            let sync_req = serde_json::json!({
                "command": "SYNC_BLOCK",
                "block": block_clone
            });
            let payload = sync_req.to_string();

            for peer in peers_clone {
                if peer != current_addr {
                    if let Ok(mut stream) = TcpStream::connect(&peer) {
                        let _ = stream.write_all(payload.as_bytes());
                    }
                }
            }
        });
    }

    pub fn broadcast_mempool_tx(tx_data: &str, peers: &std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>) {
        use std::io::Write;
        use std::net::TcpStream;
        
        let tx_clone = tx_data.to_string();
        let peers_clone: Vec<String> = peers.lock().unwrap().iter().cloned().collect();
        
        thread::spawn(move || {
            // Find current port to avoid sending to self
            let current_port = std::env::var("PORT").unwrap_or_else(|_| "7878".to_string());
            let current_addr = format!("127.0.0.1:{}", current_port);

            let sync_req = serde_json::json!({
                "command": "SYNC_MEMPOOL",
                "tx_data": tx_clone
            });
            let payload = sync_req.to_string();

            for peer in peers_clone {
                if peer != current_addr {
                    if let Ok(mut stream) = TcpStream::connect(&peer) {
                        let _ = stream.write_all(payload.as_bytes());
                    }
                }
            }
        });
    }
}
