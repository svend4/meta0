use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u128,
    pub previous_hash: String,
    pub data: String, // AST Bytecode or AIL State
    pub nonce: u64,
    pub hash: String,
}

const DIFFICULTY: usize = 3;

impl Block {
    pub fn new(index: u64, previous_hash: String, data: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        Block {
            index,
            timestamp,
            previous_hash,
            data,
            nonce: 0,
            hash: String::new(),
        }
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.index.to_string());
        hasher.update(self.timestamp.to_string());
        hasher.update(&self.previous_hash);
        hasher.update(&self.data);
        hasher.update(self.nonce.to_string());
        let result = hasher.finalize();
        let mut hash_string = String::new();
        for byte in result {
            hash_string.push_str(&format!("{:02x}", byte));
        }
        hash_string
    }

    pub fn mine_block(&mut self) {
        let prefix = "0".repeat(DIFFICULTY);
        println!("[Miner] ⛏️ Начало майнинга блока #{} (Сложность: {})...", self.index, DIFFICULTY);
        loop {
            self.hash = self.calculate_hash();
            if self.hash.starts_with(&prefix) {
                println!("[Miner] 💎 Блок добыт! Nonce: {}, Hash: {}...", self.nonce, &self.hash[0..15]);
                break;
            }
            self.nonce += 1;
            // Simulate heavy computation (optional, remove for full speed)
            // if self.nonce % 10000 == 0 {
            //     println!("[Miner] ... mining: {}", self.nonce);
            // }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AilLedger {
    pub chain: Vec<Block>,
}

impl AilLedger {
    pub fn new() -> Self {
        let port = std::env::var("PORT").unwrap_or_else(|_| "7878".to_string());
        let filename = format!("ail_ledger_{}.json", port);
        
        if let Ok(data) = fs::read_to_string(&filename) {
            if let Ok(chain) = serde_json::from_str::<Vec<Block>>(&data) {
                println!("[Ledger] 🔗 Блокчейн-Реестр загружен с диска ({} блоков)...", chain.len());
                return AilLedger { chain };
            }
        }

        println!("[Ledger] 🔗 Инициализация Блокчейн-Реестра AIL (Genesis Block)...");
        let mut ledger = AilLedger { chain: Vec::new() };
        let mut genesis_block = Block::new(0, String::from("0000000000000000"), String::from("AIL_GENESIS_OMEGA"));
        genesis_block.mine_block();
        ledger.chain.push(genesis_block);
        ledger.save_to_disk();
        ledger
    }

    pub fn save_to_disk(&self) {
        let port = std::env::var("PORT").unwrap_or_else(|_| "7878".to_string());
        let filename = format!("ail_ledger_{}.json", port);
        if let Ok(json) = serde_json::to_string_pretty(&self.chain) {
            let _ = fs::write(&filename, json);
        }
    }

    pub fn add_block(&mut self, data: String) {
        let previous_block = self.chain.last().unwrap();
        let mut new_block = Block::new(previous_block.index + 1, previous_block.hash.clone(), data);
        new_block.mine_block();
        println!("[Ledger] 🧊 Записан новый блок: #{} | Hash: {}...", new_block.index, &new_block.hash[0..10]);
        self.chain.push(new_block);
        self.save_to_disk();
    }

    pub fn verify_and_append_block(&mut self, block: Block) -> bool {
        let last_block = self.chain.last().unwrap();
        if block.index <= last_block.index {
            return false; // Блок устарел или уже есть
        }
        if block.previous_hash != last_block.hash {
            println!("[Ledger] ⚠️ Отклонен блок #{}: не совпадает хэш предыдущего блока", block.index);
            return false; // Разрыв цепи
        }
        
        let calculated_hash = block.calculate_hash();
        if block.hash != calculated_hash {
            println!("[Ledger] ⚠️ Отклонен блок #{}: неверный хэш", block.index);
            return false; // Подделка
        }
        
        let prefix = "0".repeat(DIFFICULTY);
        if !block.hash.starts_with(&prefix) {
            println!("[Ledger] ⚠️ Отклонен блок #{}: недостаточная сложность Proof of Entropy", block.index);
            return false;
        }
        
        println!("[Ledger] 🌐 Добавлен блок из P2P сети: #{} | Hash: {}...", block.index, &block.hash[0..10]);
        self.chain.push(block);
        self.save_to_disk();
        true
    }
}
