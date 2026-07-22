use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// EntangledState: Эмуляция Квантовой Запутанности памяти.
/// Вместо сетевых P2P рассылок (Gossip), память является единым макро-состоянием.
/// Модификация в одной точке пространства-времени мгновенно коллапсирует 
/// волновую функцию стейта для всех остальных узлов.

#[derive(Clone)]
pub struct EntangledState {
    // Используем потокобезопасный "Квантовый" слой памяти.
    macro_state: Arc<RwLock<HashMap<String, u64>>>,
}

impl EntangledState {
    pub fn new() -> Self {
        Self {
            macro_state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Инициализация "Волновой функции" до запуска
    pub fn init_wavefunction(&self, id: &str, balance: u64) {
        let mut state = self.macro_state.write().unwrap();
        state.insert(id.to_string(), balance);
    }

    /// Коллапс волновой функции: Мгновенное квантовое обновление баланса 
    /// (видно во всех шардах со скоростью, превышающей скорость света в эмуляции).
    pub fn collapse_and_mutate(&self, shard_id: usize, id: &str, amount: u64) {
        let mut state = self.macro_state.write().unwrap();
        state.insert(id.to_string(), amount);
        println!("[Quantum Entanglement | Shard {}] Волновая функция коллапсировала. Состояние мгновенно зафиксировано во Вселенной.", shard_id);
    }

    /// Квантовое измерение (считывание стейта)
    pub fn measure_state(&self, id: &str) -> Option<u64> {
        let state = self.macro_state.read().unwrap();
        state.get(id).cloned()
    }
}
