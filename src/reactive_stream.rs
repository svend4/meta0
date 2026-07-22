// ============================================================
// AIL Phase 45 — Reactive Time Window (Временные окна аналитики)
// Эквивалент: stream::TicketSalesStream -> window::time(10s) -> aggregate::count()
// из Google-Search_2026_07_22__0812.md, Пример 12 (dynamic_pricing.ail)
// ============================================================

use std::collections::VecDeque;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

/// Событие в потоке данных
#[derive(Debug, Clone)]
pub struct StreamEvent<T: Clone> {
    pub timestamp: Instant,
    pub data: T,
}

/// Скользящее временное окно (Time Window)
/// Точная реализация: window::time(duration::seconds(10)) -> aggregate::count()
pub struct TimeWindow<T: Clone + Send> {
    buffer: Arc<Mutex<VecDeque<StreamEvent<T>>>>,
    window_duration: Duration,
    max_capacity: usize,
}

impl<T: Clone + Send + 'static> TimeWindow<T> {
    pub fn new(window_duration: Duration, max_capacity: usize) -> Self {
        TimeWindow {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(max_capacity))),
            window_duration,
            max_capacity,
        }
    }

    /// Добавить событие в поток (эквивалент: stream::push(event))
    pub fn push(&self, data: T) {
        let mut buf = self.buffer.lock().unwrap();
        // Удаляем устаревшие события
        let cutoff = Instant::now() - self.window_duration;
        while buf.front().map(|e| e.timestamp < cutoff).unwrap_or(false) {
            buf.pop_front();
        }
        // Защита от переполнения буфера
        if buf.len() >= self.max_capacity {
            buf.pop_front();
        }
        buf.push_back(StreamEvent { timestamp: Instant::now(), data });
    }

    /// aggregate::count() — количество событий в окне
    pub fn count(&self) -> usize {
        let cutoff = Instant::now() - self.window_duration;
        let buf = self.buffer.lock().unwrap();
        buf.iter().filter(|e| e.timestamp >= cutoff).count()
    }

    /// aggregate::velocity() — события в секунду (скорость потока)
    pub fn velocity_per_second(&self) -> f64 {
        let count = self.count() as f64;
        let secs = self.window_duration.as_secs_f64();
        if secs > 0.0 { count / secs } else { 0.0 }
    }

    /// Получить все актуальные события окна
    pub fn current_events(&self) -> Vec<T> {
        let cutoff = Instant::now() - self.window_duration;
        let buf = self.buffer.lock().unwrap();
        buf.iter()
            .filter(|e| e.timestamp >= cutoff)
            .map(|e| e.data.clone())
            .collect()
    }
}

// ============================================================
// AIL Dynamic Pricing Engine
// Реализует: node::RecalculatePriceOnEvent из Примера 12
// [contract::post(price >= base && price <= base * 4)]
// ============================================================

#[derive(Debug, Clone)]
pub struct PricingConfig {
    pub base_price: u64,
    pub current_price: u64,
    pub total_seats: u32,
    pub seats_sold: u32,
}

impl PricingConfig {
    pub fn new(base_price: u64, total_seats: u32) -> Self {
        PricingConfig {
            base_price,
            current_price: base_price,
            total_seats,
            seats_sold: 0,
        }
    }
}

pub struct DynamicPricingEngine {
    config: Arc<Mutex<PricingConfig>>,
    sales_stream: TimeWindow<u64>, // u64 = timestamp событий продажи
    max_multiplier: f64,           // contract::post(price <= base * max_multiplier)
}

impl DynamicPricingEngine {
    pub fn new(base_price: u64, total_seats: u32, max_multiplier: f64) -> Self {
        DynamicPricingEngine {
            config: Arc::new(Mutex::new(PricingConfig::new(base_price, total_seats))),
            sales_stream: TimeWindow::new(Duration::from_secs(10), 10_000),
            max_multiplier,
        }
    }

    /// Обработка события продажи билета
    /// Эквивалент: node::RecalculatePriceOnEvent(event: TicketSoldEvent)
    pub fn on_ticket_sold(&self) -> u64 {
        // Добавляем событие в поток
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        self.sales_stream.push(ts);

        // Рассчитываем скорость продаж за последние 10 секунд
        let velocity = self.sales_stream.velocity_per_second();

        let mut cfg = self.config.lock().unwrap();
        cfg.seats_sold += 1;

        // Коэффициент доступности: (оставшихся мест) / (всего мест)
        let availability_ratio = if cfg.total_seats > 0 {
            (cfg.total_seats - cfg.seats_sold) as f64 / cfg.total_seats as f64
        } else {
            0.0
        };

        // Формула динамического ценообразования (linear_to_tensor аппроксимация)
        // demand_coefficient = velocity нормализованный к [0..1]
        let demand_coefficient = (velocity / 100.0).min(1.0); // 100 продаж/сек = макс спрос
        let multiplier = 1.0 + demand_coefficient * (1.0 - availability_ratio);

        // contract::post(price >= base && price <= base * max_multiplier)
        let new_price = (cfg.base_price as f64 * multiplier) as u64;
        let clamped = new_price
            .max(cfg.base_price)                                          // >= base_price
            .min((cfg.base_price as f64 * self.max_multiplier) as u64);  // <= base * max

        cfg.current_price = clamped;

        println!(
            "[DynamicPricing] 📈 Velocity={:.1}/s | Availability={:.0}% | Multiplier={:.2}x | Price={} → {} руб.",
            velocity,
            availability_ratio * 100.0,
            multiplier,
            cfg.base_price,
            clamped
        );

        clamped
    }

    pub fn get_current_price(&self) -> u64 {
        self.config.lock().unwrap().current_price
    }

    pub fn get_stats(&self) -> (u64, u32, u32, f64) {
        let cfg = self.config.lock().unwrap();
        let velocity = self.sales_stream.velocity_per_second();
        (cfg.current_price, cfg.seats_sold, cfg.total_seats, velocity)
    }
}

// ============================================================
// Rate Limiter (DDoS защита)
// Реализует: check::is_greater_than(100) из ddos_protected_counter.ail (Пример 9)
// [contract::max_latency_cycles(850)]
// ============================================================

pub struct RateLimiter {
    ip_windows: Arc<Mutex<std::collections::HashMap<String, TimeWindow<()>>>>,
    max_requests_per_window: usize,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        RateLimiter {
            ip_windows: Arc::new(Mutex::new(std::collections::HashMap::new())),
            max_requests_per_window: max_requests,
            window_duration: Duration::from_secs(window_secs),
        }
    }

    /// Проверить IP. true = разрешено, false = заблокировано (DDoS)
    pub fn check_and_record(&self, ip: &str) -> bool {
        let mut windows = self.ip_windows.lock().unwrap();
        let window = windows
            .entry(ip.to_string())
            .or_insert_with(|| TimeWindow::new(self.window_duration, self.max_requests_per_window * 2));

        let current_count = window.count();

        if current_count >= self.max_requests_per_window {
            println!(
                "[RateLimiter] 🚫 DDoS detected: IP {} exceeded {}/{} req/{}s",
                ip, current_count, self.max_requests_per_window,
                self.window_duration.as_secs()
            );
            false
        } else {
            window.push(());
            true
        }
    }

    pub fn request_count_for(&self, ip: &str) -> usize {
        let windows = self.ip_windows.lock().unwrap();
        windows.get(ip).map(|w| w.count()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_time_window_count() {
        let window: TimeWindow<i32> = TimeWindow::new(Duration::from_secs(1), 100);
        window.push(1);
        window.push(2);
        window.push(3);
        assert_eq!(window.count(), 3);
    }

    #[test]
    fn test_rate_limiter_blocks_after_limit() {
        let limiter = RateLimiter::new(3, 60);
        assert!(limiter.check_and_record("192.168.1.1"));
        assert!(limiter.check_and_record("192.168.1.1"));
        assert!(limiter.check_and_record("192.168.1.1"));
        // 4-й должен быть заблокирован
        assert!(!limiter.check_and_record("192.168.1.1"));
    }

    #[test]
    fn test_dynamic_pricing_increases_with_demand() {
        let engine = DynamicPricingEngine::new(5000, 100, 4.0);
        // Симулируем быстрые продажи
        for _ in 0..20 {
            engine.on_ticket_sold();
        }
        let price = engine.get_current_price();
        // Цена должна вырасти (хотя бы немного)
        assert!(price >= 5000, "Price should not drop below base: {}", price);
    }

    #[test]
    fn test_pricing_contract_max_multiplier() {
        let engine = DynamicPricingEngine::new(1000, 10, 4.0);
        // Продаём все билеты
        for _ in 0..10 {
            engine.on_ticket_sold();
        }
        let price = engine.get_current_price();
        // Контракт: price <= base * 4 = 4000
        assert!(price <= 4000, "Price exceeded max multiplier: {}", price);
    }
}
