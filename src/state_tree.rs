use std::collections::HashMap;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::fs;

/// AIL State Tree - Unified State Space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AilStateTree {
    global_memory: HashMap<String, f64>,
    pub root_hash: String,
}

impl AilStateTree {
    pub fn new() -> Self {
        let port = std::env::var("PORT").unwrap_or_else(|_| "7878".to_string());
        let filename = format!("ail_state_tree_{}.json", port);
        
        if let Ok(data) = fs::read_to_string(&filename) {
            if let Ok(mut tree) = serde_json::from_str::<AilStateTree>(&data) {
                println!("[StateTree] 🌳 Дерево Состояний загружено с диска (Root: {})", tree.root_hash);
                tree.recalculate_root();
                return tree;
            }
        }
        
        println!("[StateTree] 🌳 Инициализация пустого Дерева Состояний...");
        let mut tree = AilStateTree {
            global_memory: HashMap::new(),
            root_hash: String::new(),
        };
        tree.recalculate_root();
        tree
    }

    pub fn save_to_disk(&self) {
        let port = std::env::var("PORT").unwrap_or_else(|_| "7878".to_string());
        let filename = format!("ail_state_tree_{}.json", port);
        if let Ok(json) = serde_json::to_string_pretty(&self) {
            let _ = fs::write(&filename, json);
        }
    }

    pub fn recalculate_root(&mut self) {
        let mut keys: Vec<&String> = self.global_memory.keys().collect();
        keys.sort(); // Deterministic ordering
        let mut hasher = Sha256::new();
        for k in keys {
            hasher.update(k.as_bytes());
            if let Some(v) = self.global_memory.get(k) {
                hasher.update(v.to_be_bytes());
            }
        }
        let result = hasher.finalize();
        let mut hash_string = String::new();
        for byte in result {
            hash_string.push_str(&format!("{:02x}", byte));
        }
        self.root_hash = hash_string;
    }

    pub fn get(&self, key: &str) -> Option<f64> {
        self.global_memory.get(key).copied()
    }

    pub fn put(&mut self, key: String, value: f64) {
        self.global_memory.insert(key, value);
        self.recalculate_root();
    }
}

/// Software Transactional Memory wrapper
pub struct StateTransaction<'a> {
    parent_tree: &'a mut AilStateTree,
    local_changes: HashMap<String, f64>,
    aborted: bool,
}

impl<'a> StateTransaction<'a> {
    pub fn new(parent_tree: &'a mut AilStateTree) -> Self {
        StateTransaction {
            parent_tree,
            local_changes: HashMap::new(),
            aborted: false,
        }
    }

    pub fn get(&self, key: &str) -> f64 {
        if let Some(val) = self.local_changes.get(key) {
            *val
        } else {
            self.parent_tree.get(key).unwrap_or(0.0)
        }
    }

    pub fn put(&mut self, key: String, value: f64) {
        self.local_changes.insert(key, value);
    }

    pub fn abort(&mut self) {
        self.aborted = true;
        self.local_changes.clear();
        println!("[StateTransaction] 🛑 Транзакция отменена. Локальные изменения сброшены.");
    }

    pub fn commit(self) -> Result<String, String> {
        if self.aborted {
            return Err("Transaction was aborted".to_string());
        }
        println!("[StateTransaction] 💾 Коммит изменений (Software Transactional Memory)...");
        for (k, v) in self.local_changes {
            self.parent_tree.global_memory.insert(k, v);
        }
        self.parent_tree.recalculate_root();
        self.parent_tree.save_to_disk();
        println!("[StateTransaction] 🌳 Новый корень стейт-дерева: {}", self.parent_tree.root_hash);
        Ok(self.parent_tree.root_hash.clone())
    }
}
