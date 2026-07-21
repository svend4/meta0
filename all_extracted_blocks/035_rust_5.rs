use std::collections::HashMap;

// ==========================================
// 1. ОПРЕДЕЛЕНИЕ СТРУКТУРЫ AST-ГРАФА (На уровне типов Rust)
// ==========================================

#[derive(Clone)]
enum AilOperation {
    SysStateBind { target_store: String },
    MapGetProperty { property: String },
    MathSubSafeProof,
    ForwardExit { exit_code: u16 },
    ConditionalBranch {
        on_success: Box<AilOperation>,
        on_overflow: Box<AilOperation>,
    },
}

struct AstRoot {
    node_id: String,
    body_nodes: Vec<AilOperation>,
}

// ==========================================
// 2. ДВИЖОК ВИРТУАЛЬНОЙ МАШИНЫ (AIL Runtime Core)
// ==========================================

struct AilVirtualMachine {
    // Имитируем Единое Распределенное Пространство Состояний в ОЗУ
    state_space: HashMap<String, HashMap<String, u64>>,
}

impl AilVirtualMachine {
    fn new() -> Self {
        let mut state_space = HashMap::new();
        let mut ledger = HashMap::new();

        // Создаем кошелек с начальным балансом 1000 единиц
        ledger.insert("wallet-user-777".to_string(), 1000);
        state_space.insert("GlobalLedger".to_string(), ledger);

        AilVirtualMachine { state_space }
    }

    // Исполнение скомпилированного графа операций
    fn execute_node(&mut self, ast: &AstRoot, wallet_id: &str, input_amount: u64) -> (u16, String) {
        let mut current_balance = 0;
        let mut is_bound = false;
        let mut target_store_name = String::new();

        // Проходим по топологически отсортированным узлам графа
        for node in &ast.body_nodes {
            match node {
                AilOperation::SysStateBind { target_store } => {
                    if self.state_space.contains_key(target_store) &&
                       self.state_space[target_store].contains_key(wallet_id) {
                        is_bound = true;
                        target_store_name = target_store.clone();
                    } else {
                        return (404, "Runtime Error: Context address not found".to_string());
                    }
                }

                AilOperation::MapGetProperty { property } => {
                    if is_bound && property == "balance" {
                        current_balance = self.state_space[&target_store_name][wallet_id];
                    }
                }

                AilOperation::MathSubSafeProof => {
                    // Математическая верификация инварианта безопасности на лету
                    let invariant_proven = current_balance >= input_amount;

                    // Поиск узла ветвления (он всегда идет следом в нашей архитектуре)
                    if let Some(AilOperation::ConditionalBranch { on_success, on_overflow }) = ast.body_nodes.last() {
                        if invariant_proven {
                            // Безопасная мутация состояния напрямую в памяти (Zero-Copy)
                            if let AilOperation::ConditionalBranch { on_success: success_op, .. } = node_eval(on_success) {
                                if let AilOperation::ForwardExit { exit_code } = **success_op {
                                    if let Some(ledger) = self.state_space.get_mut(&target_store_name) {
                                        if let Some(balance) = ledger.get_mut(wallet_id) {
                                            *balance -= input_amount; // Списание
                                        }
                                    }
                                    return (exit_code, format!("Executed. New balance: {}", current_balance - input_amount));
                                }
                            }
                        } else {
                            // Инвариант нарушен: мгновенный откат и выход по ветке Overflow
                            if let AilOperation::ForwardExit { exit_code } = **on_overflow {
                                return (exit_code, "Rejected: Insufficient funds (Safety Invariant Violated)".to_string());
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        (500, "Internal Execution Error".to_string())
    }
}

// Вспомогательная функция для ленивого разбора ссылок
fn node_eval(op: &Box<AilOperation>) -> AilOperation {
    (**op).clone()
}

// ==========================================
// 3. ТОЧКА ВХОДА И ТЕСТИРОВАНИЕ СКОРОСТИ
// ==========================================

fn main() {
    println!("--- ЗАПУСК ПРОМЫШЛЕННОГО ЯДРА AIL НА RUST (ИЮЛЬ 2026) --- \n");

    // Ручной синтез AST-графа (Именно так его собирает ИИ-компилятор)
    let ast_program = AstRoot {
        node_id: "ND_001".to_string(),
        body_nodes: vec![
            AilOperation::SysStateBind { target_store: "GlobalLedger".to_string() },
            AilOperation::MapGetProperty { property: "balance".to_string() },
            AilOperation::MathSubSafeProof,
            AilOperation::ConditionalBranch {
                on_success: Box::new(AilOperation::ForwardExit { exit_code: 200 }),
                on_overflow: Box::new(AilOperation::ForwardExit { exit_code: 402 }),
            },
        ],
    };

    let mut ail_vm = AilVirtualMachine::new();

    // Запрос 1: Попытка списать 400 единиц (Безопасно)
    print!("Запрос 1: Списание 400 единиц... ");
    let (code_1, msg_1) = ail_vm.execute_node(&ast_program, "wallet-user-777", 400);
    println!("Ответ ВМ: Код [{}] -> {}", code_1, msg_1);

    // Запрос 2: Попытка списать 700 единиц (Остаток 600 -> Ошибка!)
    print!("Запрос 2: Списание 700 единиц... ");
    let (code_2, msg_2) = ail_vm.execute_node(&ast_program, "wallet-user-777", 700);
    println!("Ответ ВМ: Код [{}] -> {}", code_2, msg_2);
}