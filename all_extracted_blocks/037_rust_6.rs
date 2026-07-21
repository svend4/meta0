use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ==========================================
// 1. СЕРИАЛИЗАЦИЯ И ДЕСЕРИАЛИЗАЦИЯ AST-ГРАФА
// ==========================================

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "operation")] // Позволяет мапить JSON-поле "operation" в варианты Enum
enum AilOperation {
    #[serde(rename = "SYS_STATE_BIND")]
    SysStateBind { target_store: String },

    #[serde(rename = "MAP_GET_PROPERTY")]
    MapGetProperty { property: String },

    #[serde(rename = "MATH_SUB_SAFE_PROOF")]
    MathSubSafeProof,

    #[serde(rename = "FORWARD_EXIT")]
    ForwardExit { exit_code: u16 },

    #[serde(rename = "CONDITIONAL_BRANCH")]
    ConditionalBranch {
        branches: HashMap<String, AilOperation>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct BodyGraph {
    nodes: HashMap<String, AilOperation>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AstRoot {
    node_id: String,
    body_graph: BodyGraph,
}

#[derive(Serialize, Deserialize, Debug)]
struct AilAstEnvelope {
    ast_root: AstRoot,
}

// ==========================================
// 2. СРЕДА ИСПОЛНЕНИЯ (AIL Virtual Machine)
// ==========================================

struct AilVirtualMachine {
    state_space: HashMap<String, HashMap<String, u64>>,
}

impl AilVirtualMachine {
    fn new() -> Self {
        let mut state_space = HashMap::new();
        let mut ledger = HashMap::new();
        ledger.insert("wallet-user-777".to_string(), 1000); // Баланс 1000
        state_space.insert("GlobalLedger".to_string(), ledger);

        AilVirtualMachine { state_space }
    }

    fn execute_json_ast(&mut self, json_str: &str, wallet_id: &str, input_amount: u64) -> (u16, String) {
        // Парсинг JSON напрямую в структуры Rust (минуя текстовую компиляцию кода)
        let envelope: AilAstEnvelope = match serde_json::from_str(json_str) {
            Ok(env) => env,
            Err(e) => return (500, format!("AST Parsing Failure: {}", e)),
        };

        let nodes = &envelope.ast_root.body_graph.nodes;

        let mut current_balance = 0;
        let mut is_bound = false;
        let mut target_store_name = String::new();

        // Имитируем топологический обход графа по ключам-операциям
        // Шаг 1: Привязка к стейту
        if let Some(AilOperation::SysStateBind { target_store }) = nodes.get("OP_BindState") {
            if self.state_space.contains_key(target_store) &&
               self.state_space[target_store].contains_key(wallet_id) {
                is_bound = true;
                target_store_name = target_store.clone();
            } else {
                return (404, "Runtime Error: State address context not found".to_string());
            }
        }

        // Шаг 2: Извлечение свойства баланса
        if let Some(AilOperation::MapGetProperty { property }) = nodes.get("OP_GetBalance") {
            if is_bound && property == "balance" {
                current_balance = self.state_space[&target_store_name][wallet_id];
            }
        }

        // Шаг 3: Верификация инварианта безопасности математики и Ветвление
        if nodes.contains_key("OP_CheckInvariant") {
            let invariant_proven = current_balance >= input_amount;

            if let Some(AilOperation::ConditionalBranch { branches }) = nodes.get("BRANCH_Decision") {
                if invariant_proven {
                    // Ветка успешного списания
                    if let Some(AilOperation::SysStateBind { .. }) = Some(AilOperation::SysStateBind { target_store: "".to_string() }) { // Проводка мутации
                        if let Some(ledger) = self.state_space.get_mut(&target_store_name) {
                            if let Some(balance) = ledger.get_mut(wallet_id) {
                                *balance -= input_amount;
                            }
                        }
                        return (200, format!("Success transaction. New balance: {}", current_balance - input_amount));
                    }
                } else {
                    // Ветка блокировки при риске овердрафта
                    if let Some(AilOperation::ForwardExit { exit_code }) = branches.get("on_overflow") {
                        return (*exit_code, "Rejected: Safety invariant violated (Insufficient funds)".to_string());
                    }
                }
            }
        }

        (500, "Internal Execution Contract Fault".to_string())
    }
}

// ==========================================
// 3. ТЕСТИРОВАНИЕ СКВОЗНОГО ПРОТОТИПА
# // ==========================================

fn main() {
    println!("--- СКВОЗНОЙ ТЕСТ ИНТЕГРАЦИИ JSON -> RUST-ЯДРО AIL --- \n");

    // Этот JSON-текст — точная копия того, что сгенерировал наш скрипт на Python!
    let incoming_ai_json = r#"
    {
      "ast_root": {
        "node_id": "ND_001",
        "body_graph": {
          "nodes": {
            "OP_BindState": { "operation": "SYS_STATE_BIND", "target_store": "GlobalLedger" },
            "OP_GetBalance": { "operation": "MAP_GET_PROPERTY", "property": "balance" },
            "OP_CheckInvariant": { "operation": "MATH_SUB_SAFE_PROOF" },
            "BRANCH_Decision": {
              "operation": "CONDITIONAL_BRANCH",
              "branches": {
                "on_overflow": { "operation": "FORWARD_EXIT", "exit_code": 402 }
              }
            }
          }
        }
      }
    }
    "#;

    let mut ail_vm = AilVirtualMachine::new();

    // Тест 1: Списание 250 единиц (Валидно)
    println!("Вызов 1: Попытка провести транзакцию на 250 единиц...");
    let (code_1, res_1) = ail_vm.execute_json_ast(incoming_ai_json, "wallet-user-777", 250);
    println!("Результат Rust-ядра: [{}] -> {}\n", code_1, res_1);

    // Тест 2: Списание 900 единиц (Остаток 750 -> Должно заблокироваться)
    println!("Вызов 2: Попытка провести транзакцию на 900 единиц...");
    let (code_2, res_2) = ail_vm.execute_json_ast(incoming_ai_json, "wallet-user-777", 900);
    println!("Результат Rust-ядра: [{}] -> {}", code_2, res_2);
}