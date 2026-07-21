use std::collections::HashMap;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone)]
pub struct AilWallet {
    pub address: String,
    pub private_key: String,
    pub public_key: String,
    pub balance: u64,
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

pub struct WalletManager {
    wallets: HashMap<String, AilWallet>,
}

impl WalletManager {
    pub fn new() -> Self {
        WalletManager {
            wallets: HashMap::new(),
        }
    }

    pub fn create_wallet(&mut self, seed: &str) -> String {
        let wallet = AilWallet::new(seed);
        let address = wallet.address.clone();
        self.wallets.insert(address.clone(), wallet);
        address
    }

    pub fn get_wallet_mut(&mut self, address: &str) -> Option<&mut AilWallet> {
        self.wallets.get_mut(address)
    }
    
    pub fn get_balance(&self, address: &str) -> u64 {
        self.wallets.get(address).map(|w| w.balance).unwrap_or(0)
    }
}
