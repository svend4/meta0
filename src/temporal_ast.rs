use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// TemporalLedger: Квантовая память с путешествием во времени
/// Хранит версии состояний без дублирования (Event-Sourcing на микроуровне)
pub struct TemporalLedger {
    history: Vec<StateSnapshot>,
    current_state: HashMap<String, u64>,
}

#[derive(Clone)]
pub struct StateSnapshot {
    pub timestamp: u128,
    pub changes: HashMap<String, u64>, // Снимок измененных ключей для O(1) отката
}

impl TemporalLedger {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            current_state: HashMap::new(),
        }
    }

    /// Инициализация баланса (до транзакций)
    pub fn init_balance(&mut self, id: &str, balance: u64) {
        self.current_state.insert(id.to_string(), balance);
    }

    /// Попытка мутации состояния с фиксацией слепка (snapshot)
    pub fn mutate_balance(&mut self, id: &str, new_balance: u64) {
        let old_balance = self.current_state.get(id).cloned().unwrap_or(0);
        let mut changes = HashMap::new();
        changes.insert(id.to_string(), old_balance); // Запоминаем старое значение для отката

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        self.history.push(StateSnapshot { timestamp, changes });
        self.current_state.insert(id.to_string(), new_balance);
    }

    /// Откат последней транзакции за O(1) (Quantum Rollback)
    pub fn rollback_last_mutation(&mut self) -> Result<(), &'static str> {
        if let Some(snapshot) = self.history.pop() {
            for (key, old_value) in snapshot.changes {
                self.current_state.insert(key, old_value);
            }
            Ok(())
        } else {
            Err("No mutations to rollback")
        }
    }

    pub fn get_balance(&self, id: &str) -> Option<u64> {
        self.current_state.get(id).cloned()
    }
}
