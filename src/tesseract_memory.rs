use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// TesseractMemory: 4D Гиперкуб для хранения данных.
/// Баланс существует в параллельных реальностях до момента измерения Наблюдателем.

#[derive(Clone)]
pub struct TesseractState {
    // Вектор реальностей (X, Y, Z, W) -> HashMap стейтов
    hyper_planes: Arc<RwLock<Vec<HashMap<String, u64>>>>,
}

impl TesseractState {
    pub fn new() -> Self {
        let mut planes = Vec::new();
        for _ in 0..4 { // 4 измерения Тессеракта
            planes.push(HashMap::new());
        }
        Self {
            hyper_planes: Arc::new(RwLock::new(planes)),
        }
    }

    pub fn inject_anomaly(&self, id: &str, amount: u64) {
        let mut planes = self.hyper_planes.write().unwrap();
        // Данные пишутся во все параллельные плоскости одновременно
        for plane in planes.iter_mut() {
            plane.insert(id.to_string(), amount);
        }
        println!("[Tesseract Memory] Данные записаны в 4D Гиперкуб. Состояние существует в суперпозиции.");
    }
}
