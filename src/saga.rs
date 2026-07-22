// ============================================================
// AIL Phase 45 — Saga Orchestrator (Распределённые транзакции)
// Эквивалент: Паттерн "Сага" из Google-Search_2026_07_22__0812.md
// Пример 11: concert_payment_gateway.ail
// Реализует: Initiated -> BankProcessing -> Completed | Compensated
// ============================================================

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Статус шага саги
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SagaStepStatus {
    Pending,
    Executing,
    Completed,
    Failed(String),
    Compensating,
    Compensated,
}

/// Одиночный шаг саги с логикой прямого действия и компенсации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    pub step_id: String,
    pub description: String,
    pub status: SagaStepStatus,
    pub executed_at: Option<u64>,   // Unix timestamp (nanos)
    pub compensated_at: Option<u64>,
}

/// Тип результата саги
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SagaResult {
    Committed,                    // Все шаги выполнены успешно
    Compensated { failed_step: String, reason: String }, // Частичный откат
    Failed { step: String, error: String },              // Критическая ошибка
}

/// Лог мутации состояния (Event Sourcing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaEvent {
    pub saga_id: String,
    pub step_id: String,
    pub event_type: String,   // "STEP_START" | "STEP_OK" | "STEP_FAIL" | "COMPENSATE_OK"
    pub timestamp_ns: u128,
    pub payload: Option<String>,
}

/// Оркестратор саги — обеспечивает согласованность распределённых транзакций
/// Реализует паттерн из файла: TransactionState { Initiated, BankProcessing, Completed, Compensated }
pub struct SagaOrchestrator {
    pub saga_id: String,
    pub steps: Vec<SagaStep>,
    /// Функции прямого действия: step_id -> Box<dyn FnOnce() -> Result<String, String>>
    execute_fns: Vec<Box<dyn Fn() -> Result<String, String>>>,
    /// Функции компенсации (откат): step_id -> Box<dyn FnOnce() -> Result<(), String>>
    compensate_fns: Vec<Box<dyn Fn() -> Result<(), String>>>,
    pub event_log: Vec<SagaEvent>,
}

impl SagaOrchestrator {
    pub fn new(saga_id: &str) -> Self {
        SagaOrchestrator {
            saga_id: saga_id.to_string(),
            steps: Vec::new(),
            execute_fns: Vec::new(),
            compensate_fns: Vec::new(),
            event_log: Vec::new(),
        }
    }

    /// Добавить шаг в сагу
    /// execute_fn — основное действие
    /// compensate_fn — откат этого действия если последующий шаг провалится
    pub fn add_step<E, C>(
        &mut self,
        step_id: &str,
        description: &str,
        execute_fn: E,
        compensate_fn: C,
    ) where
        E: Fn() -> Result<String, String> + 'static,
        C: Fn() -> Result<(), String> + 'static,
    {
        self.steps.push(SagaStep {
            step_id: step_id.to_string(),
            description: description.to_string(),
            status: SagaStepStatus::Pending,
            executed_at: None,
            compensated_at: None,
        });
        self.execute_fns.push(Box::new(execute_fn));
        self.compensate_fns.push(Box::new(compensate_fn));
    }

    fn log_event(&mut self, step_id: &str, event_type: &str, payload: Option<String>) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        self.event_log.push(SagaEvent {
            saga_id: self.saga_id.clone(),
            step_id: step_id.to_string(),
            event_type: event_type.to_string(),
            timestamp_ns: ts,
            payload,
        });
    }

    /// Выполнить сагу: прямые действия + автоматическая компенсация при ошибке.
    /// Это эквивалент поведения компилятора AIL при директиве @state_model::event_sourced
    pub fn execute(&mut self) -> SagaResult {
        println!("\n[Saga::{}] 🚀 Запуск распределённой транзакции ({} шагов)", 
            self.saga_id, self.steps.len());

        let mut completed_steps: Vec<usize> = Vec::new();

        // === ФАЗА ПРЯМОГО ВЫПОЛНЕНИЯ ===
        for i in 0..self.steps.len() {
            let step_id = self.steps[i].step_id.clone();
            let description = self.steps[i].description.clone();

            println!("[Saga::{}] ⚙️  Шаг [{}]: {}", self.saga_id, step_id, description);
            self.steps[i].status = SagaStepStatus::Executing;
            self.log_event(&step_id, "STEP_START", None);

            match (self.execute_fns[i])() {
                Ok(output) => {
                    self.steps[i].status = SagaStepStatus::Completed;
                    self.log_event(&step_id, "STEP_OK", Some(output.clone()));
                    println!("[Saga::{}] ✅ Шаг [{}] выполнен: {}", self.saga_id, step_id, output);
                    completed_steps.push(i);
                }
                Err(reason) => {
                    self.steps[i].status = SagaStepStatus::Failed(reason.clone());
                    self.log_event(&step_id, "STEP_FAIL", Some(reason.clone()));
                    println!("[Saga::{}] ❌ Шаг [{}] провалился: {}", self.saga_id, step_id, reason);

                    // === ФАЗА КОМПЕНСАЦИИ (Откат выполненных шагов в обратном порядке) ===
                    println!("[Saga::{}] 🔄 Запуск компенсации ({} шагов к откату)...", 
                        self.saga_id, completed_steps.len());

                    for &comp_i in completed_steps.iter().rev() {
                        let comp_step_id = self.steps[comp_i].step_id.clone();
                        self.steps[comp_i].status = SagaStepStatus::Compensating;
                        println!("[Saga::{}] ↩️  Компенсация шага [{}]...", self.saga_id, comp_step_id);

                        match (self.compensate_fns[comp_i])() {
                            Ok(_) => {
                                self.steps[comp_i].status = SagaStepStatus::Compensated;
                                self.log_event(&comp_step_id, "COMPENSATE_OK", None);
                                println!("[Saga::{}] ✅ Шаг [{}] успешно откатан.", self.saga_id, comp_step_id);
                            }
                            Err(comp_err) => {
                                // Критическая ошибка — нарушена согласованность!
                                println!(
                                    "[Saga::{}] 🚨 КРИТИЧНО: Компенсация шага [{}] провалилась: {}",
                                    self.saga_id, comp_step_id, comp_err
                                );
                                self.log_event(&comp_step_id, "COMPENSATE_FAIL", Some(comp_err.clone()));
                                return SagaResult::Failed {
                                    step: comp_step_id,
                                    error: format!("Compensation failed: {}", comp_err),
                                };
                            }
                        }
                    }

                    return SagaResult::Compensated {
                        failed_step: step_id,
                        reason,
                    };
                }
            }
        }

        println!("[Saga::{}] 🎉 Все шаги выполнены. Транзакция зафиксирована.", self.saga_id);
        SagaResult::Committed
    }

    /// Экспорт лога событий (для ИИ-аудита и Event Sourcing)
    pub fn export_event_log_json(&self) -> String {
        serde_json::to_string_pretty(&self.event_log).unwrap_or_else(|_| "[]".to_string())
    }

    /// Сводка по всем шагам
    pub fn print_summary(&self) {
        println!("\n[Saga::{}] 📋 СВОДКА:", self.saga_id);
        for step in &self.steps {
            let icon = match &step.status {
                SagaStepStatus::Completed => "✅",
                SagaStepStatus::Compensated => "↩️",
                SagaStepStatus::Failed(_) => "❌",
                SagaStepStatus::Compensating => "🔄",
                SagaStepStatus::Executing => "⚙️",
                SagaStepStatus::Pending => "⏳",
            };
            println!("  {} [{}] {} — {:?}", icon, step.step_id, step.description, step.status);
        }
    }
}

// ============================================================
// Фабричный метод — создание саги для платёжного шлюза
// Точная реализация concert_payment_gateway.ail из анализа
// ============================================================

/// Создаёт сагу для бронирования и оплаты билета
/// Реализует: Initiated -> BankProcessing -> Completed | Compensated
pub fn build_ticket_payment_saga(
    tx_id: &str,
    seat_id: u32,
    user_id: u64,
    amount: u64,
    seat_state: std::sync::Arc<std::sync::Mutex<HashMap<u32, String>>>,
    ledger_state: std::sync::Arc<std::sync::Mutex<HashMap<String, String>>>,
) -> SagaOrchestrator {
    let mut saga = SagaOrchestrator::new(tx_id);

    // Шаг 1: Зарезервировать место за пользователем
    let seat_state_1 = std::sync::Arc::clone(&seat_state);
    let seat_state_comp_1 = std::sync::Arc::clone(&seat_state);
    saga.add_step(
        "RESERVE_SEAT",
        &format!("Резервация места #{} для user#{}", seat_id, user_id),
        move || {
            let mut seats = seat_state_1.lock().unwrap();
            if seats.get(&seat_id).map(|s| s.as_str()) == Some("AVAILABLE") {
                seats.insert(seat_id, format!("RESERVED_BY_{}", user_id));
                Ok(format!("Seat {} reserved for user {}", seat_id, user_id))
            } else {
                Err(format!("Seat {} is not available", seat_id))
            }
        },
        move || {
            let mut seats = seat_state_comp_1.lock().unwrap();
            seats.insert(seat_id, "AVAILABLE".to_string());
            println!("[Saga][Compensate] Место #{} возвращено в продажу", seat_id);
            Ok(())
        },
    );

    // Шаг 2: Записать транзакцию как "Инициировано"
    let ledger_2 = std::sync::Arc::clone(&ledger_state);
    let ledger_comp_2 = std::sync::Arc::clone(&ledger_state);
    let tx_id_2 = tx_id.to_string();
    let tx_id_comp_2 = tx_id.to_string();
    saga.add_step(
        "LOG_INITIATED",
        "Запись транзакции в PaymentLedger (Initiated)",
        move || {
            let mut ledger = ledger_2.lock().unwrap();
            ledger.insert(tx_id_2.clone(), format!("INITIATED:user={},amount={}", user_id, amount));
            Ok(format!("Transaction {} logged as INITIATED", tx_id_2))
        },
        move || {
            let mut ledger = ledger_comp_2.lock().unwrap();
            ledger.remove(&tx_id_comp_2);
            println!("[Saga][Compensate] Запись транзакции удалена из лога");
            Ok(())
        },
    );

    // Шаг 3: Вызов внешнего банка (имитация)
    let ledger_3 = std::sync::Arc::clone(&ledger_state);
    let ledger_comp_3 = std::sync::Arc::clone(&ledger_state);
    let seat_state_3 = std::sync::Arc::clone(&seat_state);
    let tx_id_3 = tx_id.to_string();
    let tx_id_comp_3 = tx_id.to_string();
    saga.add_step(
        "BANK_CHARGE",
        &format!("Списание {} у банка для user#{}", amount, user_id),
        move || {
            // Имитируем вызов банка — в реальном коде здесь CircuitBreaker
            // Для демонстрации: суммы > 50000 "отклоняются банком"
            if amount > 50_000 {
                return Err("Bank declined: insufficient credit limit".to_string());
            }
            let mut ledger = ledger_3.lock().unwrap();
            ledger.insert(tx_id_3.clone(), format!("COMPLETED:amount={}", amount));
            Ok(format!("Bank charged {} for tx {}", amount, tx_id_3))
        },
        move || {
            // Откат: возвращаем деньги, освобождаем место
            let mut ledger = ledger_comp_3.lock().unwrap();
            ledger.insert(tx_id_comp_3.clone(), "COMPENSATED:reason=bank_failed".to_string());
            let mut seats = seat_state_3.lock().unwrap();
            seats.insert(seat_id, "AVAILABLE".to_string());
            println!("[Saga][Compensate] Возврат средств и освобождение места #{}", seat_id);
            Ok(())
        },
    );

    saga
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn make_test_state() -> (Arc<Mutex<HashMap<u32, String>>>, Arc<Mutex<HashMap<String, String>>>) {
        let mut seats = HashMap::new();
        seats.insert(42u32, "AVAILABLE".to_string());
        (
            Arc::new(Mutex::new(seats)),
            Arc::new(Mutex::new(HashMap::new())),
        )
    }

    #[test]
    fn test_saga_commits_successfully() {
        let (seats, ledger) = make_test_state();
        let mut saga = build_ticket_payment_saga(
            "tx-001", 42, 1, 1000, Arc::clone(&seats), Arc::clone(&ledger)
        );
        let result = saga.execute();
        assert_eq!(result, SagaResult::Committed);
        let seats_guard = seats.lock().unwrap();
        assert_eq!(seats_guard.get(&42).unwrap(), "RESERVED_BY_1");
    }

    #[test]
    fn test_saga_compensates_on_bank_failure() {
        let (seats, ledger) = make_test_state();
        // amount > 50000 вызовет ошибку банка
        let mut saga = build_ticket_payment_saga(
            "tx-002", 42, 2, 99_999, Arc::clone(&seats), Arc::clone(&ledger)
        );
        let result = saga.execute();
        // Должна быть компенсация
        assert!(matches!(result, SagaResult::Compensated { .. }));
        // Место должно вернуться в AVAILABLE
        let seats_guard = seats.lock().unwrap();
        assert_eq!(seats_guard.get(&42).unwrap(), "AVAILABLE");
    }
}
