use std::collections::HashMap;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AilWallet {
    pub address: String,
    pub private_key: String,
    pub public_key: String,
    pub balance: u64,
    pub tokens: HashMap<String, u64>,
    pub quantum_states_owned: Vec<String>,
}

impl AilWallet {
    pub fn new(seed: &str) -> Self {
        // Generating pseudo-keys for AIL Phase 11
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        hasher.update(b"PRIVATE_KEY_ENTROPY");
        let priv_hash = hasher.finalize();
        
        let mut priv_str = String::new();
        for byte in priv_hash { priv_str.push_str(&format!("{:02x}", byte)); }
        
        let mut pub_hasher = Sha256::new();
        pub_hasher.update(priv_str.as_bytes());
        pub_hasher.update(b"PUBLIC_KEY_DERIVATION");
        let pub_hash = pub_hasher.finalize();
        
        let mut pub_str = String::new();
        for byte in pub_hash { pub_str.push_str(&format!("{:02x}", byte)); }
        
        let address = format!("AIL-{}", &pub_str[0..16]);
        
        AilWallet {
            address,
            private_key: priv_str,
            public_key: pub_str,
            balance: 100, // Genesis balance for new wallets
            tokens: HashMap::new(),
            quantum_states_owned: Vec::new(),
        }
    }

    pub fn receive_tokens(&mut self, amount: u64) {
        self.balance += amount;
    }

    pub fn spend_tokens(&mut self, amount: u64) -> Result<(), String> {
        if self.balance >= amount {
            self.balance -= amount;
            Ok(())
        } else {
            Err("Insufficient AIL balance".to_string())
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletManager {
    pub wallets: HashMap<String, AilWallet>,
}

impl WalletManager {
    pub fn new() -> Self {
        let port = std::env::var("PORT").unwrap_or_else(|_| "7878".to_string());
        let filename = format!("ail_wallets_{}.json", port);
        
        if let Ok(data) = fs::read_to_string(&filename) {
            if let Ok(wallets) = serde_json::from_str::<HashMap<String, AilWallet>>(&data) {
                println!("[Wallet] 💼 База кошельков загружена с диска ({} кошельков)...", wallets.len());
                return WalletManager { wallets };
            }
        }

        WalletManager {
            wallets: HashMap::new(),
        }
    }

    pub fn save_to_disk(&self) {
        let port = std::env::var("PORT").unwrap_or_else(|_| "7878".to_string());
        let filename = format!("ail_wallets_{}.json", port);
        if let Ok(json) = serde_json::to_string_pretty(&self.wallets) {
            let _ = fs::write(&filename, json);
        }
    }

    pub fn create_wallet(&mut self, seed: &str) -> String {
        let wallet = AilWallet::new(seed);
        let address = wallet.address.clone();
        self.wallets.insert(address.clone(), wallet);
        self.save_to_disk();
        address
    }

    pub fn get_wallet_mut(&mut self, address: &str) -> Option<&mut AilWallet> {
        self.wallets.get_mut(address)
    }
    
    pub fn get_balance(&self, address: &str) -> u64 {
        self.wallets.get(address).map(|w| w.balance).unwrap_or(0)
    }

    pub fn transfer_tokens(&mut self, from_address: &str, to_address: &str, amount: u64) -> Result<(), String> {
        let sender = self.wallets.get_mut(from_address).ok_or("Sender wallet not found")?;
        if sender.balance < amount {
            return Err("Insufficient balance".to_string());
        }
        sender.balance -= amount;

        if !self.wallets.contains_key(to_address) {
            // Create a pseudo-wallet if it doesn't exist to accept tokens
            let mut new_wallet = AilWallet::new(to_address); // use address as seed
            new_wallet.address = to_address.to_string(); // force address match
            new_wallet.balance = 0; // Don't give Genesis balance to pseudo-wallets
            self.wallets.insert(to_address.to_string(), new_wallet);
        }

        let receiver = self.wallets.get_mut(to_address).unwrap();
        receiver.balance += amount;

        self.save_to_disk();
        Ok(())
    }

    pub fn mint_custom_token(&mut self, address: &str, token_name: &str, amount: u64) -> Result<(), String> {
        let wallet = self.wallets.get_mut(address).ok_or("Wallet not found")?;
        let balance = wallet.tokens.entry(token_name.to_string()).or_insert(0);
        *balance += amount;
        self.save_to_disk();
        Ok(())
    }

    pub fn transfer_custom_token(&mut self, from_address: &str, to_address: &str, token_name: &str, amount: u64) -> Result<(), String> {
        let sender = self.wallets.get_mut(from_address).ok_or("Sender not found")?;
        let sender_balance = sender.tokens.get_mut(token_name).ok_or("Token not found on sender")?;
        
        if *sender_balance < amount {
            return Err("Insufficient token balance".to_string());
        }
        *sender_balance -= amount;

        if !self.wallets.contains_key(to_address) {
            let mut new_wallet = AilWallet::new(to_address);
            new_wallet.address = to_address.to_string();
            new_wallet.balance = 0;
            self.wallets.insert(to_address.to_string(), new_wallet);
        }

        let receiver = self.wallets.get_mut(to_address).unwrap();
        let receiver_balance = receiver.tokens.entry(token_name.to_string()).or_insert(0);
        *receiver_balance += amount;

        self.save_to_disk();
        Ok(())
    }
}
