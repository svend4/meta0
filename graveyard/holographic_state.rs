use std::collections::{HashMap, HashSet};

/// VectorClock: Для определения причинно-следственных связей в P2P сети
#[derive(Debug, Clone)]
pub struct VectorClock {
    pub node_clocks: HashMap<usize, u64>,
}

impl Default for VectorClock {
    fn default() -> Self {
        Self {
            node_clocks: HashMap::new(),
        }
    }
}

/// HolographicState: Состояние, которое "размазано" по сети.
/// Вместо центральной базы, каждый узел хранит голограмму и обменивается диффами.
pub struct HolographicState {
    pub node_id: usize,
    pub clock: VectorClock,
    pub known_peers: HashSet<usize>,
    pub local_ledger: HashMap<String, u64>,
}

impl HolographicState {
    pub fn new(node_id: usize) -> Self {
        Self {
            node_id,
            clock: VectorClock::default(),
            known_peers: HashSet::new(),
            local_ledger: HashMap::new(),
        }
    }

    pub fn discover_peer(&mut self, peer_id: usize) {
        self.known_peers.insert(peer_id);
    }

    /// Эмуляция Gossip (Эпидемического) протокола распространения
    pub fn broadcast_mutation(&mut self, wallet_id: &str, amount: u64) {
        // Увеличиваем логические часы узла
        let current_tick = self.clock.node_clocks.entry(self.node_id).or_insert(0);
        *current_tick += 1;

        // В реальной системе здесь будет отправка UDP Multicast или TCP потока к known_peers
        println!("[Gossip P2P | Node {}] Широковещательная рассылка состояния для '{}'. Tick: {}", self.node_id, wallet_id, current_tick);
        self.local_ledger.insert(wallet_id.to_string(), amount);
    }
    
    pub fn sync_from_network(&mut self, wallet_id: &str, amount: u64, sender_node: usize) {
        println!("[Gossip P2P | Node {}] Получена синхронизация от Node {}. Обновление стейта.", self.node_id, sender_node);
        self.local_ledger.insert(wallet_id.to_string(), amount);
    }
}
