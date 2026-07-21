use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u128,
    pub previous_hash: String,
    pub data: String, // AST Bytecode or AIL State
    pub hash: String,
}

impl Block {
    pub fn new(index: u64, previous_hash: String, data: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        let mut block = Block {
            index,
            timestamp,
            previous_hash,
            data,
            hash: String::new(),
        };
        block.hash = block.calculate_hash();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.index.to_string());
        hasher.update(self.timestamp.to_string());
        hasher.update(&self.previous_hash);
        hasher.update(&self.data);
        let result = hasher.finalize();
        let mut hash_string = String::new();
        for byte in result {
            hash_string.push_str(&format!("{:02x}", byte));
        }
        hash_string
    }
}

pub struct AilLedger {
    pub chain: Vec<Block>,
}

impl AilLedger {
    pub fn new() -> Self {
        println!("[Ledger] 🔗 Инициализация Блокчейн-Реестра AIL (Genesis Block)...");
        let mut ledger = AilLedger { chain: Vec::new() };
        let genesis_block = Block::new(0, String::from("0000000000000000"), String::from("AIL_GENESIS_OMEGA"));
        ledger.chain.push(genesis_block);
        ledger
    }

    pub fn add_block(&mut self, data: String) {
        let previous_block = self.chain.last().unwrap();
        let new_block = Block::new(previous_block.index + 1, previous_block.hash.clone(), data);
        println!("[Ledger] 🧊 Записан новый блок: #{} | Hash: {}...", new_block.index, &new_block.hash[0..10]);
        self.chain.push(new_block);
    }
}
