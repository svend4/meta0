/// SemanticRouter: Маршрутизация на основе семантики AST, а не хэшей ID
/// 
/// "Понимает" сложность и риск транзакции, распределяя тяжелые графы 
/// на защищенные (или более производительные) шарды.

pub struct SemanticRouter {
    shard_count: usize,
}

#[derive(Debug)]
pub enum TransactionRisk {
    Low(usize),    // Легкая транзакция (сразу на кэш-ядра)
    High(usize),   // Тяжелая/рискованная транзакция (на защищенные ядра)
}

impl SemanticRouter {
    pub fn new(shard_count: usize) -> Self {
        Self { shard_count }
    }

    /// Анализ вектора транзакции (Embeddings)
    /// В реальной AIL это математически доказанное семантическое дерево.
    /// В прототипе мы эмулируем анализ "веса" транзакции по её структуре/сумме.
    pub fn analyze_and_route(&self, amount: u64, is_cross_border: bool) -> TransactionRisk {
        // Эмуляция ИИ-модели семантического анализа
        let risk_score = if is_cross_border { amount * 2 } else { amount };
        
        if risk_score > 5000 {
            // Тяжелые и рискованные операции отправляются на последние 2 шарда (High-Security)
            let shard = (self.shard_count - 1) - (amount as usize % 2);
            TransactionRisk::High(shard)
        } else {
            // Быстрые легкие операции отправляются на первые шарды (Fast-Cache)
            let shard = amount as usize % (self.shard_count - 2).max(1);
            TransactionRisk::Low(shard)
        }
    }
}
