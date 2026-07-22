// ============================================================
// AIL Phase 45 — Circuit Breaker (Защита от каскадного падения)
// Эквивалент: [circuit_breaker::max_failures(3), ::cooldown(30s)]
// из Google-Search_2026_07_22__0812.md, Пример 11
// ============================================================

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

/// Состояние автоматического выключателя (предохранителя)
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BreakerState {
    Closed,    // Всё нормально, пропускаем запросы
    Open,      // Слишком много ошибок — блокируем запросы
    HalfOpen,  // Cooldown прошёл — пробуем 1 запрос
}

#[derive(Debug)]
pub enum CircuitBreakerError {
    CircuitOpen { cooldown_remaining_ms: u64 },
    CallFailed(String),
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen { cooldown_remaining_ms } =>
                write!(f, "Circuit OPEN — Предохранитель разомкнут. Ожидание: {}ms", cooldown_remaining_ms),
            CircuitBreakerError::CallFailed(msg) =>
                write!(f, "Внешний вызов завершился ошибкой: {}", msg),
        }
    }
}

/// AIL Circuit Breaker — математически гарантирует изоляцию падений внешних систем
/// Прямая реализация концепции из файла:
/// [circuit_breaker::max_failures(3), circuit_breaker::cooldown(duration::seconds(30))]
pub struct CircuitBreaker {
    name: String,
    failure_count: AtomicU32,
    max_failures: u32,
    cooldown: Duration,
    state: AtomicU8,                         // 0=Closed, 1=Open, 2=HalfOpen
    last_failure_time: Arc<Mutex<Option<Instant>>>,
    total_calls: AtomicU32,
    total_successes: AtomicU32,
    total_rejections: AtomicU32,
}

impl CircuitBreaker {
    pub fn new(name: &str, max_failures: u32, cooldown_secs: u64) -> Arc<Self> {
        Arc::new(CircuitBreaker {
            name: name.to_string(),
            failure_count: AtomicU32::new(0),
            max_failures,
            cooldown: Duration::from_secs(cooldown_secs),
            state: AtomicU8::new(0), // Closed
            last_failure_time: Arc::new(Mutex::new(None)),
            total_calls: AtomicU32::new(0),
            total_successes: AtomicU32::new(0),
            total_rejections: AtomicU32::new(0),
        })
    }

    fn get_state(&self) -> BreakerState {
        match self.state.load(Ordering::SeqCst) {
            0 => BreakerState::Closed,
            1 => BreakerState::Open,
            _ => BreakerState::HalfOpen,
        }
    }

    fn set_state(&self, new_state: BreakerState) {
        let code = match new_state {
            BreakerState::Closed   => 0,
            BreakerState::Open     => 1,
            BreakerState::HalfOpen => 2,
        };
        self.state.store(code, Ordering::SeqCst);
    }

    fn cooldown_remaining_ms(&self) -> u64 {
        let guard = self.last_failure_time.lock().unwrap();
        if let Some(t) = *guard {
            let elapsed = t.elapsed();
            if elapsed < self.cooldown {
                let remaining = self.cooldown - elapsed;
                return remaining.as_millis() as u64;
            }
        }
        0
    }

    /// Выполнить внешний вызов через предохранитель.
    /// Автоматически управляет состоянием на основе результата.
    pub fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::fmt::Display,
    {
        self.total_calls.fetch_add(1, Ordering::Relaxed);

        // Проверяем текущее состояние
        match self.get_state() {
            BreakerState::Open => {
                let remaining = self.cooldown_remaining_ms();
                if remaining > 0 {
                    self.total_rejections.fetch_add(1, Ordering::Relaxed);
                    println!("[CircuitBreaker::{}] 🔴 OPEN — запрос отклонён. Cooldown: {}ms", self.name, remaining);
                    return Err(CircuitBreakerError::CircuitOpen { cooldown_remaining_ms: remaining });
                } else {
                    // Cooldown прошёл — переходим в HalfOpen для пробного запроса
                    self.set_state(BreakerState::HalfOpen);
                    println!("[CircuitBreaker::{}] 🟡 HALF-OPEN — пробный запрос...", self.name);
                }
            }
            BreakerState::Closed => {}
            BreakerState::HalfOpen => {
                println!("[CircuitBreaker::{}] 🟡 HALF-OPEN — пробуем восстановление...", self.name);
            }
        }

        // Выполняем вызов
        match f() {
            Ok(result) => {
                // Успех — сбрасываем счётчик ошибок
                self.failure_count.store(0, Ordering::SeqCst);
                self.set_state(BreakerState::Closed);
                self.total_successes.fetch_add(1, Ordering::Relaxed);
                println!("[CircuitBreaker::{}] ✅ CLOSED — вызов успешен.", self.name);
                Ok(result)
            }
            Err(e) => {
                // Фиксируем ошибку
                let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                {
                    let mut t = self.last_failure_time.lock().unwrap();
                    *t = Some(Instant::now());
                }

                if failures >= self.max_failures {
                    self.set_state(BreakerState::Open);
                    println!(
                        "[CircuitBreaker::{}] 🔴 OPEN — {} ошибок подряд! Блокировка на {}s.",
                        self.name, failures, self.cooldown.as_secs()
                    );
                } else {
                    println!(
                        "[CircuitBreaker::{}] ⚠️ Ошибка {}/{}: {}",
                        self.name, failures, self.max_failures, e
                    );
                }

                Err(CircuitBreakerError::CallFailed(e.to_string()))
            }
        }
    }

    /// Статистика работы предохранителя
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            name: self.name.clone(),
            state: self.get_state(),
            failure_count: self.failure_count.load(Ordering::Relaxed),
            total_calls: self.total_calls.load(Ordering::Relaxed),
            total_successes: self.total_successes.load(Ordering::Relaxed),
            total_rejections: self.total_rejections.load(Ordering::Relaxed),
            cooldown_remaining_ms: self.cooldown_remaining_ms(),
        }
    }

    /// Принудительный сброс (для тестирования)
    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        self.set_state(BreakerState::Closed);
        let mut t = self.last_failure_time.lock().unwrap();
        *t = None;
        println!("[CircuitBreaker::{}] 🔄 Предохранитель принудительно сброшен.", self.name);
    }
}

#[derive(Debug)]
pub struct CircuitBreakerStats {
    pub name: String,
    pub state: BreakerState,
    pub failure_count: u32,
    pub total_calls: u32,
    pub total_successes: u32,
    pub total_rejections: u32,
    pub cooldown_remaining_ms: u64,
}

impl std::fmt::Display for CircuitBreakerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,
            "[CircuitBreaker::{}] State={:?} | Failures={}/{} | Calls={} | OK={} | Rejected={}",
            self.name, self.state, self.failure_count,
            // max_failures not stored here, display what we have
            "N",
            self.total_calls, self.total_successes, self.total_rejections
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_opens_after_max_failures() {
        let cb = CircuitBreaker::new("test_bank", 3, 30);
        // Три провала
        for _ in 0..3 {
            let _ = cb.call::<_, (), _>(|| Err("bank down"));
        }
        // Четвёртый вызов должен быть отклонён предохранителем
        match cb.call::<_, (), &str>(|| Ok(())) {
            Err(CircuitBreakerError::CircuitOpen { .. }) => {} // Ожидаемо
            other => panic!("Expected CircuitOpen, got {:?}", other),
        }
    }

    #[test]
    fn test_circuit_closes_on_success() {
        let cb = CircuitBreaker::new("test_recovery", 2, 0); // cooldown=0 для теста
        let _ = cb.call::<_, (), _>(|| Err("error 1"));
        let _ = cb.call::<_, (), _>(|| Err("error 2"));
        // Сразу проверяем HalfOpen (cooldown=0)
        let result = cb.call::<_, i32, &str>(|| Ok(42));
        assert!(result.is_ok());
    }
}
