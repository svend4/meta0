use std::collections::HashMap;
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};
use serde::{Deserialize, Serialize};

// ==========================================
// 1. СТРУКТУРЫ ДАННЫХ И БАЙТ-КОД
// ==========================================

#[derive(Serialize, Deserialize, Debug, Clone)]
enum AilBinaryOp {
    SysStateBind { target_store: String },
    MapGetProperty { property: String },
    MathSubSafeProof,
    ConditionalBranch { on_overflow_code: u16 },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BinaryAstPayload {
    node_id: u32,
    operations: Vec<AilBinaryOp>,
}

struct ExecutionTask {
    payload: BinaryAstPayload,
    wallet_id: String,
    amount: u64,
    response_sender: Sender<String>,
}

// ==========================================
// 2. ВЫЧИСЛИТЕЛЬНОЕ ЯДРО ШАРДА (Shard Actor)
// ==========================================

struct AilShardActor {
    shard_id: usize,
    local_ledger: HashMap<String, u64>,
    task_receiver: Receiver<ExecutionTask>,
}

impl AilShardActor {
    fn new(shard_id: usize, task_receiver: Receiver<ExecutionTask>) -> Self {
        let mut local_ledger = HashMap::new();
        // Заполняем тестовыми данными кошельки этого шарда
        local_ledger.insert(format!("wallet-user-{}", shard_id), 50000);

        AilShardActor { shard_id, local_ledger, task_receiver }
    }

    fn run_loop(&mut self) {
        while let Ok(task) = self.task_receiver.recv() {
            let mut current_balance = 0;
            let mut is_bound = false;
            let mut execution_success = false;
            let mut status_msg = String::new();

            for op in &task.payload.operations {
                match op {
                    AilBinaryOp::SysStateBind { target_store } => {
                        if target_store == "GlobalLedger" {
                            is_bound = true;
                        }
                    }
                    AilBinaryOp::MapGetProperty { property } => {
                        if is_bound && property == "balance" {
                            current_balance = *self.local_ledger.get(&task.wallet_id).unwrap_or(&0);
                        }
                    }
                    AilBinaryOp::MathSubSafeProof => {
                        if current_balance >= task.amount {
                            if let Some(balance) = self.local_ledger.get_mut(&task.wallet_id) {
                                *balance -= task.amount;
                            }
                            execution_success = true;
                            status_msg = format!("SUCCESS on Shard {}. Balance: {}", self.shard_id, current_balance - task.amount);
                        } else {
                            status_msg = format!("REJECTED on Shard {}: Low Funds", self.shard_id);
                        }
                    }
                    AilBinaryOp::ConditionalBranch { on_overflow_code } => {
                        let final_code = if execution_success { 200 } else { *on_overflow_code };
                        let _ = task.response_sender.send(format!("CODE: {}; {}", final_code, status_msg));
                    }
                }
            }
        }
    }
}

// ==========================================
// 3. МАРШРУТИЗАТОР КЛАСТЕРА (Cluster Router)
// ==========================================

struct AilClusterRouter {
    shard_channels: Vec<Sender<ExecutionTask>>,
    num_shards: usize,
}

impl AilClusterRouter {
    fn new(num_shards: usize) -> (Self, Vec<Receiver<ExecutionTask>>) {
        let mut shard_channels = Vec::new();
        let mut receivers = Vec::new();

        for _ in 0..num_shards {
            let (tx, rx) = unbounded::<ExecutionTask>();
            shard_channels.push(tx);
            receivers.push(rx);
        }

        (AilClusterRouter { shard_channels, num_shards }, receivers)
    }

    // Хэширование и отправка задачи на нужное ядро процессора
    fn route_task(&self, task: ExecutionTask) {
        // Простейший алгоритм распределения по бакетам на основе длины строки ID
        // В реальном AIL тут используется сверхбыстрый некриптографический хэш (например, MurmurHash3)
        let shard_index = task.wallet_id.len() % self.num_shards;

        // Отправляем задачу в lock-free канал конкретного актора
        self.shard_channels[shard_index].send(task).unwrap();
    }
}

// ==========================================
// 4. ЗАПУСК И ДЕМОНСТРАЦИЯ РАБОТЫ В КЛАСТЕРЕ
// ==========================================

fn main() {
    println!("--- ВЕРИФИКАЦИЯ МАРШРУТИЗАТОРА И ПУЛА АКТОРОВ AIL (2026) --- \n");

    let num_cores = 4; // Симулируем работу на 4-ядерном процессоре
    let (router, receivers) = AilClusterRouter::new(num_cores);

    // Запускаем 4 изолированных потока (по одному Актору на ядро)
    for (i, rx) in receivers.into_iter().enumerate() {
        thread::spawn(move || {
            let mut actor = AilShardActor::new(i, rx);
            actor.run_loop();
        });
    }
    println!("[System]: Успешно развернуто {} независимых вычислительных ядер. \n", num_cores);

    let (tx_res, rx_res) = unbounded::<String>();

    // Имитируем прилет двух разных транзакций для разных кошельков
    let payload = BinaryAstPayload {
        node_id: 1002,
        operations: vec![
            AilBinaryOp::SysStateBind { target_store: "GlobalLedger".to_string() },
            AilBinaryOp::MapGetProperty { property: "balance".to_string() },
            AilBinaryOp::MathSubSafeProof,
            AilBinaryOp::ConditionalBranch { on_overflow_code: 402 },
        ],
    };

    // Транзакция 1: Попадет на Шард 1 (длина строки "wallet-user-1" = 13 % 4 = 1)
    println!("[Router]: Маршрутизация Транзакции 1 для 'wallet-user-1'...");
    router.route_task(ExecutionTask {
        payload: payload.clone(),
        wallet_id: "wallet-user-1".to_string(),
        amount: 5000,
        response_sender: tx_res.clone(),
    });
    println!("[Ответ Сети]: {}", rx_res.recv().unwrap());

    // Транзакция 2: Попадет на Шард 2 (длина строки "wallet-user-2" = 13 % 4 = 1, изменим длину для наглядности)
    // "wallet-user-33" = 14 % 4 = 2 -> Шард 2
    println!("\n[Router]: Маршрутизация Транзакции 2 для 'wallet-user-33'...");
    router.route_task(ExecutionTask {
        payload: payload.clone(),
        wallet_id: "wallet-user-33".to_string(), // Попадет на Шард 2 (создан в пуле по умолчанию)
        amount: 10000,
        response_sender: tx_res,
    });
    println!("[Ответ Сети]: {}", rx_res.recv().unwrap());
}