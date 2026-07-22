// ============================================================
// AIL Проход 2 — Shard-Actor Engine + Binary Transport
// Честный перенос идей из Google-Search_2026_07_22__0812.md
// (супер-ядра v1-v4: lock-free акторы, шардирующий роутер, bincode-транспорт),
// но БЕЗ дефектов лога: реальный hash-роутинг вместо `len % n`,
// монопольное владение памятью шарда, детерминированная сериализация задач.
//
// Ключевая гарантия: каждый кошелёк принадлежит ровно одному шард-актору.
// Все операции над ним сериализуются в его crossbeam-очереди, поэтому
// овердрафт невозможен БЕЗ единого мьютекса — за счёт топологии, а не блокировок.
// ============================================================

use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

// ── Байткод графа (bincode-сериализуемый) ────────────────────────────────
// Тот же набор опкодов, что в логе (SysStateBind → MapGetProperty →
// MathSubSafeProof → ConditionalBranch), но интерпретируется корректно.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AilBinaryOp {
    SysStateBind { store: String },
    MapGetProperty { property: String },
    MathSubSafeProof,
    ConditionalBranch { on_overflow_code: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryAstPayload {
    pub node_id: u32,
    pub operations: Vec<AilBinaryOp>,
}

impl BinaryAstPayload {
    /// Канонический граф списания: bind → get balance → safe-sub proof → branch(402).
    pub fn debit() -> Self {
        BinaryAstPayload {
            node_id: 1,
            operations: vec![
                AilBinaryOp::SysStateBind { store: "GlobalLedger".to_string() },
                AilBinaryOp::MapGetProperty { property: "balance".to_string() },
                AilBinaryOp::MathSubSafeProof,
                AilBinaryOp::ConditionalBranch { on_overflow_code: 402 },
            ],
        }
    }
}

/// Запрос списания, приходящий по бинарному TCP-транспорту (bincode).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryTxRequest {
    pub wallet_id: String,
    pub amount: u64,
    pub payload: BinaryAstPayload,
}

/// Результат транзакции (bincode-сериализуемый — уходит обратно клиенту).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxResult {
    pub code: u16,
    pub message: String,
    pub new_balance: u64,
    pub shard_id: usize,
}

// ── Сообщения актору ─────────────────────────────────────────────────────
pub struct ExecutionTask {
    pub payload: BinaryAstPayload,
    pub wallet_id: String,
    pub amount: u64,
    pub reply: Sender<TxResult>,
}

pub enum ActorMessage {
    Execute(ExecutionTask),
    Seed { wallet_id: String, balance: u64, ack: Sender<()> },
    Balance { wallet_id: String, reply: Sender<Option<u64>> },
}

// ── Шард-актор: владеет своим куском леджера единолично ──────────────────
struct ShardActor {
    shard_id: usize,
    ledger: HashMap<String, u64>,
    rx: Receiver<ActorMessage>,
}

impl ShardActor {
    fn run(mut self) {
        // Атомарное ожидание задач; вся сериализация доступа к памяти — здесь.
        while let Ok(msg) = self.rx.recv() {
            match msg {
                ActorMessage::Seed { wallet_id, balance, ack } => {
                    self.ledger.insert(wallet_id, balance);
                    let _ = ack.send(());
                }
                ActorMessage::Balance { wallet_id, reply } => {
                    let _ = reply.send(self.ledger.get(&wallet_id).copied());
                }
                ActorMessage::Execute(task) => {
                    let result = self.execute(&task);
                    let _ = task.reply.send(result);
                }
            }
        }
    }

    fn execute(&mut self, task: &ExecutionTask) -> TxResult {
        // 1) on_overflow-код берём из ветвления заранее (порядок опкодов не важен).
        let on_overflow = task
            .payload
            .operations
            .iter()
            .find_map(|op| match op {
                AilBinaryOp::ConditionalBranch { on_overflow_code } => Some(*on_overflow_code),
                _ => None,
            })
            .unwrap_or(402);

        // 2) Требуем явную привязку к GlobalLedger (SysStateBind).
        let has_bind = task.payload.operations.iter().any(|op| {
            matches!(op, AilBinaryOp::SysStateBind { store } if store == "GlobalLedger")
        });
        if !has_bind {
            return TxResult {
                code: 400,
                message: format!("Shard {}: missing SysStateBind(GlobalLedger)", self.shard_id),
                new_balance: 0,
                shard_id: self.shard_id,
            };
        }

        // 3) Читаем баланс (MapGetProperty). Нет кошелька → 404.
        let balance = match self.ledger.get(&task.wallet_id) {
            Some(b) => *b,
            None => {
                return TxResult {
                    code: 404,
                    message: format!("Shard {}: wallet {} not found", self.shard_id, task.wallet_id),
                    new_balance: 0,
                    shard_id: self.shard_id,
                }
            }
        };

        // 4) MathSubSafeProof: инвариант balance >= amount доказывается ДО мутации.
        if task.amount > balance {
            return TxResult {
                code: on_overflow,
                message: format!("Shard {}: REJECTED — insufficient funds ({} > {})", self.shard_id, task.amount, balance),
                new_balance: balance,
                shard_id: self.shard_id,
            };
        }

        // 5) Инвариант держится → мутация (единоличная, без гонок).
        let new_balance = balance - task.amount;
        self.ledger.insert(task.wallet_id.clone(), new_balance);
        TxResult {
            code: 200,
            message: format!("Shard {}: SUCCESS", self.shard_id),
            new_balance,
            shard_id: self.shard_id,
        }
    }
}

// ── Шардирующий роутер кластера ──────────────────────────────────────────
pub struct ClusterRouter {
    shard_channels: Vec<Sender<ActorMessage>>,
    num_shards: usize,
    handles: Vec<thread::JoinHandle<()>>,
}

impl ClusterRouter {
    pub fn new(num_shards: usize) -> Self {
        let num_shards = num_shards.max(1);
        let mut shard_channels = Vec::with_capacity(num_shards);
        let mut handles = Vec::with_capacity(num_shards);
        for shard_id in 0..num_shards {
            let (tx, rx) = unbounded::<ActorMessage>();
            shard_channels.push(tx);
            let actor = ShardActor { shard_id, ledger: HashMap::new(), rx };
            handles.push(thread::spawn(move || actor.run()));
        }
        ClusterRouter { shard_channels, num_shards, handles }
    }

    /// Честное шардирование: DefaultHasher(wallet_id) % num_shards.
    /// (В логе было `wallet_id.len() % n` — коллизии по длине строки, чинится здесь.)
    fn shard_for(&self, wallet_id: &str) -> usize {
        let mut h = DefaultHasher::new();
        wallet_id.hash(&mut h);
        (h.finish() % self.num_shards as u64) as usize
    }

    pub fn num_shards(&self) -> usize {
        self.num_shards
    }

    pub fn shard_index(&self, wallet_id: &str) -> usize {
        self.shard_for(wallet_id)
    }

    /// Создать/пополнить кошелёк на его шарде (синхронно, с ack).
    pub fn seed_wallet(&self, wallet_id: &str, balance: u64) {
        let (ack_tx, ack_rx) = unbounded();
        let idx = self.shard_for(wallet_id);
        let _ = self.shard_channels[idx].send(ActorMessage::Seed {
            wallet_id: wallet_id.to_string(),
            balance,
            ack: ack_tx,
        });
        let _ = ack_rx.recv();
    }

    pub fn balance_of(&self, wallet_id: &str) -> Option<u64> {
        let (tx, rx) = unbounded();
        let idx = self.shard_for(wallet_id);
        let _ = self.shard_channels[idx].send(ActorMessage::Balance {
            wallet_id: wallet_id.to_string(),
            reply: tx,
        });
        rx.recv().ok().flatten()
    }

    /// Отправить транзакцию и дождаться ответа от владеющего шарда.
    pub fn submit(&self, wallet_id: &str, amount: u64, payload: BinaryAstPayload) -> TxResult {
        let (tx, rx) = unbounded();
        let idx = self.shard_for(wallet_id);
        let task = ExecutionTask {
            payload,
            wallet_id: wallet_id.to_string(),
            amount,
            reply: tx,
        };
        let _ = self.shard_channels[idx].send(ActorMessage::Execute(task));
        rx.recv().unwrap_or(TxResult {
            code: 500,
            message: "actor channel dropped".to_string(),
            new_balance: 0,
            shard_id: idx,
        })
    }

    /// Отправить только Sender задачи в нужный шард (для конкурентных клиентов).
    pub fn dispatch(&self, wallet_id: &str, amount: u64, payload: BinaryAstPayload, reply: Sender<TxResult>) {
        let idx = self.shard_for(wallet_id);
        let task = ExecutionTask {
            payload,
            wallet_id: wallet_id.to_string(),
            amount,
            reply,
        };
        let _ = self.shard_channels[idx].send(ActorMessage::Execute(task));
    }
}

impl Drop for ClusterRouter {
    fn drop(&mut self) {
        // Закрываем каналы (Sender'ы дропаются) — акторы выйдут из recv-циклов.
        self.shard_channels.clear();
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
    }
}

// ── Бинарный транспорт (Binary Delta Streaming из лога, по-честному) ─────
// Кадр: [u32 LE длина][bincode(BinaryTxRequest)] → [u32 LE длина][bincode(TxResult)].
// В логе сервер обрабатывал ровно одно соединение и хардкодил параметры
// транзакции; здесь — вечный accept-цикл, поток на соединение, параметры
// приходят в самом кадре.

const MAX_FRAME: u32 = 1_048_576; // 1 МБ

fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let len = u32::from_le_bytes(len_buf);
    if len == 0 || len > MAX_FRAME {
        return Err(format!("frame length {} out of bounds", len));
    }
    let mut payload = vec![0u8; len as usize];
    stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
    Ok(payload)
}

pub fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> Result<(), String> {
    let len = payload.len() as u32;
    stream.write_all(&len.to_le_bytes()).map_err(|e| e.to_string())?;
    stream.write_all(payload).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn read_frame_pub(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    read_frame(stream)
}

/// Поднять бинарный листенер шард-движка. Успешные транзакции уходят в mempool
/// (и далее — в блокчейн фоновым майнером ноды).
pub fn spawn_binary_listener(
    router: Arc<ClusterRouter>,
    port: u16,
    mempool: Option<Arc<std::sync::Mutex<Vec<String>>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let addr = format!("127.0.0.1:{}", port);
        let listener = match TcpListener::bind(&addr) {
            Ok(l) => l,
            Err(e) => {
                println!("[ShardEngine] ❌ Не удалось открыть бинарный порт {}: {}", addr, e);
                return;
            }
        };
        println!("[ShardEngine] 🔩 Binary Delta Streaming слушает {} (кадры: u32 LE + bincode)", addr);

        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let router = Arc::clone(&router);
            let mempool = mempool.clone();
            thread::spawn(move || {
                let result = match read_frame(&mut stream) {
                    Ok(bytes) => match bincode::deserialize::<BinaryTxRequest>(&bytes) {
                        Ok(req) => {
                            let res = router.submit(&req.wallet_id, req.amount, req.payload);
                            if res.code == 200 {
                                if let Some(mp) = &mempool {
                                    let tx = format!(
                                        "BINARY_TX: SUB {} FROM {} (shard {}, new_balance {})",
                                        req.amount, req.wallet_id, res.shard_id, res.new_balance
                                    );
                                    mp.lock().unwrap().push(tx);
                                }
                            }
                            res
                        }
                        Err(e) => TxResult {
                            code: 500,
                            message: format!("AST_CORRUPTION: {}", e),
                            new_balance: 0,
                            shard_id: 0,
                        },
                    },
                    Err(e) => TxResult {
                        code: 400,
                        message: format!("FRAME_ERROR: {}", e),
                        new_balance: 0,
                        shard_id: 0,
                    },
                };
                if let Ok(bytes) = bincode::serialize(&result) {
                    let _ = write_frame(&mut stream, &bytes);
                }
            });
        }
    })
}

// ── Тесты ────────────────────────────────────────────────────────────────
// Гоночный тест (идея из лога, реализованная поверх настоящего акторного ядра).
// В bin-крейте держим его как unit-тест: интеграционные тесты в tests/ требуют
// lib.rs, чего у ноды нет (именно на этом спотыкался код лога).
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn debit_payload_shape() {
        let p = BinaryAstPayload::debit();
        assert_eq!(p.operations.len(), 4);
        assert!(matches!(p.operations[0], AilBinaryOp::SysStateBind { .. }));
    }

    #[test]
    fn seed_and_balance_roundtrip() {
        let router = ClusterRouter::new(4);
        router.seed_wallet("wallet-user-777", 1000);
        assert_eq!(router.balance_of("wallet-user-777"), Some(1000));
        assert_eq!(router.balance_of("nonexistent"), None);
    }

    #[test]
    fn single_debit_success_and_reject() {
        let router = ClusterRouter::new(4);
        router.seed_wallet("w1", 500);
        let ok = router.submit("w1", 400, BinaryAstPayload::debit());
        assert_eq!(ok.code, 200);
        assert_eq!(ok.new_balance, 100);
        let rej = router.submit("w1", 300, BinaryAstPayload::debit());
        assert_eq!(rej.code, 402); // 100 < 300 → овердрафт отклонён
        assert_eq!(router.balance_of("w1"), Some(100));
    }

    #[test]
    fn bincode_roundtrip() {
        let req = BinaryTxRequest {
            wallet_id: "w1".to_string(),
            amount: 250,
            payload: BinaryAstPayload::debit(),
        };
        let bytes = bincode::serialize(&req).unwrap();
        let back: BinaryTxRequest = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back.wallet_id, "w1");
        assert_eq!(back.amount, 250);
        assert_eq!(back.payload.operations.len(), 4);
    }

    // Сквозной тест транспорта: кадр по TCP → актор → кадр-ответ.
    #[test]
    fn binary_transport_end_to_end() {
        let router = Arc::new(ClusterRouter::new(2));
        router.seed_wallet("net-wallet", 5000);
        let _h = spawn_binary_listener(Arc::clone(&router), 17931, None);
        std::thread::sleep(std::time::Duration::from_millis(300));

        let mut stream = TcpStream::connect("127.0.0.1:17931").expect("connect binary port");
        let req = BinaryTxRequest {
            wallet_id: "net-wallet".to_string(),
            amount: 1500,
            payload: BinaryAstPayload::debit(),
        };
        let bytes = bincode::serialize(&req).unwrap();
        write_frame(&mut stream, &bytes).unwrap();
        let resp_bytes = read_frame_pub(&mut stream).unwrap();
        let res: TxResult = bincode::deserialize(&resp_bytes).unwrap();
        assert_eq!(res.code, 200);
        assert_eq!(res.new_balance, 3500);
    }

    // Гвоздевой тест: 15 конкурентных потоков списывают по 1000 из 10 000.
    // Инвариант «нельзя уйти в минус» должен дать РОВНО 10 успехов и 5 отказов —
    // не из-за блокировок, а из-за сериализации задач одним шард-актором.
    #[test]
    fn race_condition_invariant_holds() {
        let router = Arc::new(ClusterRouter::new(4));
        router.seed_wallet("battle-wallet", 10_000);

        let mut handles = Vec::new();
        let (result_tx, result_rx) = unbounded::<TxResult>();
        for _ in 0..15 {
            let r = Arc::clone(&router);
            let rtx = result_tx.clone();
            handles.push(thread::spawn(move || {
                r.dispatch("battle-wallet", 1000, BinaryAstPayload::debit(), rtx);
            }));
        }
        drop(result_tx);
        for h in handles {
            let _ = h.join();
        }

        let mut success = 0;
        let mut reject = 0;
        while let Ok(res) = result_rx.recv() {
            match res.code {
                200 => success += 1,
                402 => reject += 1,
                other => panic!("unexpected code {}", other),
            }
        }
        assert_eq!(success, 10, "ровно 10 списаний по 1000 должны пройти из 10000");
        assert_eq!(reject, 5, "остальные 5 должны быть отклонены");
        assert_eq!(router.balance_of("battle-wallet"), Some(0));
    }
}
