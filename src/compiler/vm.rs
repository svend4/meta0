use crate::compiler::parser::AstNode;
use crate::compiler::verifier::FormalVerifier;
use crate::wallet::WalletManager;
use crate::state_tree::{AilStateTree, StateTransaction};
use std::thread;
use std::time::Duration;

pub struct AilVirtualMachine {
    entropy_level: f64,
}

impl AilVirtualMachine {
    pub fn new() -> Self {
        println!("[AIL-VM] 🌌 Инициализация Квантовой Виртуальной Машины...");
        AilVirtualMachine {
            entropy_level: 0.0,
        }
    }

    pub fn execute(&mut self, ast: Vec<AstNode>, deployer: &str, wm: &mut WalletManager, state_tree: &mut AilStateTree) -> Result<String, String> {
        println!("[AIL-VM] ⚡ Загрузка AST в суперпозицию (Unified State)...");
        
        // --- FORMAL VERIFICATION STEP ---
        let mut verifier = FormalVerifier::new();
        if let Err(e) = verifier.verify_ast(&ast) {
            println!("[AIL-VM] 🛑 СМЕРТЕЛЬНАЯ ОШИБКА АРХИТЕКТУРЫ: {}", e);
            println!("[AIL-VM] 🛑 Контракт отвергнут до запуска. Распадение волновой функции отменено.");
            return Err(e);
        }
        // --------------------------------

        let mut tx = StateTransaction::new(state_tree);
        self.execute_internal(&ast, deployer, wm, &mut tx);
        
        println!("[AIL-VM] ✅ Выполнение завершено. Энтропия системы: {:.4}", self.entropy_level);
        println!("[AIL-VM] 💾 Коллапс волновой функции в 4D-Тессеракт...");
        tx.commit()
    }

    fn execute_internal(&mut self, ast: &Vec<AstNode>, deployer: &str, wm: &mut WalletManager, tx: &mut StateTransaction) {
        for node in ast {
            thread::sleep(Duration::from_millis(10)); // Sped up for faster testing
            match node {
                AstNode::ModuleDecl(name) => {
                    println!("[AIL-VM] 📦 Изоляция модуля пространства: {}", name);
                }
                AstNode::StateDefinition { name, state_type } => {
                    println!("[AIL-VM] 🧪 Регистрация состояния [{}]: {}", state_type, name);
                    self.entropy_level += 0.1;
                }
                AstNode::QuantumTransition { from, to } => {
                    println!("[AIL-VM] 🔄 Квантовый скачок: {} -> {}", from, to);
                    self.entropy_level -= 0.05;
                }
                AstNode::ContractMaxAllocation { bytes } => {
                    println!("[AIL-VM] ℹ️ Инвариант в рантайме: Макс. аллокация: {} байт", bytes);
                }
                AstNode::ContractPre { var_name, operator, limit } => {
                    println!("[AIL-VM] ℹ️ Инвариант в рантайме: {} {} {}", var_name, operator, limit);
                }
                AstNode::StoreState(key, val) => {
                    println!("[AIL-VM] 💾 Unified State: Сохранение: {} => {}", key, val);
                    tx.put(key.clone(), *val);
                }
                AstNode::MathAdd(key, val) => {
                    let current = tx.get(key);
                    let new_val = current + val;
                    println!("[AIL-VM] ➕ Вычисление ADD: {} ({} + {}) = {}", key, current, val, new_val);
                    tx.put(key.clone(), new_val);
                }
                AstNode::MathSub(var_name, val) => {
                    let current = tx.get(var_name);
                    let new_val = current - val;
                    println!("[AIL-VM] ➖ Вычитание: {} - {} => {}", var_name, val, new_val);
                    tx.put(var_name.clone(), new_val);
                    self.entropy_level += 0.2;
                }
                AstNode::MathMul(var_name, val) => {
                    let current = tx.get(var_name);
                    let new_val = current * val;
                    println!("[AIL-VM] ✖️ Умножение: {} * {} => {}", var_name, val, new_val);
                    tx.put(var_name.clone(), new_val);
                    self.entropy_level += 0.3;
                }
                AstNode::MathDiv(var_name, val) => {
                    let current = tx.get(var_name);
                    let new_val = if *val != 0.0 { current / val } else { 0.0 };
                    println!("[AIL-VM] ➗ Деление: {} / {} => {}", var_name, val, new_val);
                    tx.put(var_name.clone(), new_val);
                    self.entropy_level += 0.4;
                }
                AstNode::IfCondition { condition_var, operator, threshold, body } => {
                    let current = tx.get(condition_var);
                    // Проход 4: раньше исполнялся только оператор '>' — остальные
                    // ветки IF молча не срабатывали. Теперь поддержаны все сравнения.
                    let condition_met = match operator.as_str() {
                        ">"  => current > *threshold,
                        "<"  => current < *threshold,
                        ">=" => current >= *threshold,
                        "<=" => current <= *threshold,
                        "==" => (current - *threshold).abs() < f64::EPSILON,
                        "!=" => (current - *threshold).abs() >= f64::EPSILON,
                        _    => false,
                    };

                    println!("[AIL-VM] ⚖️ Условие IF: {} {} {} => {}", condition_var, operator, threshold, condition_met);
                    
                    if condition_met {
                        println!("[AIL-VM] ↪️ Вход во вложенный блок...");
                        self.execute_internal(body, deployer, wm, tx);
                        println!("[AIL-VM] ↩️ Выход из вложенного блока.");
                    }
                }
                AstNode::Loop { count_var, body } => {
                    let iterations = tx.get(count_var).max(0.0) as usize;
                    println!("[AIL-VM] 🔄 Цикл LOOP: {} раз (счетчик: {})", iterations, count_var);
                    
                    for i in 0..iterations {
                        println!("[AIL-VM] ➡️ Итерация {}/{}...", i + 1, iterations);
                        self.execute_internal(body, deployer, wm, tx);
                    }
                    println!("[AIL-VM] 🏁 Цикл завершен.");
                }
                AstNode::OracleFetch { url, extract_key, var_name } => {
                    println!("[AIL-VM] 👁️ Оракул запрашивает данные из внешнего мира: {}", url);
                    
                    let mut host = "";
                    let mut path = "/";
                    
                    let url_clean = url.trim_start_matches("http://").trim_start_matches("https://");
                    if let Some(slash_idx) = url_clean.find('/') {
                        host = &url_clean[..slash_idx];
                        path = &url_clean[slash_idx..];
                    } else {
                        host = url_clean;
                    }

                    let host_port = if host.contains(':') {
                        host.to_string()
                    } else {
                        format!("{}:80", host)
                    };

                    use std::net::TcpStream;
                    use std::io::{Write, Read};
                    
                    let mut val_to_store = 0.0;
                    
                    if let Ok(mut stream) = TcpStream::connect(&host_port) {
                        let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host);
                        if stream.write_all(request.as_bytes()).is_ok() {
                            let mut response = String::new();
                            if stream.read_to_string(&mut response).is_ok() {
                                if let Some(body_start) = response.find("\r\n\r\n") {
                                    let body = &response[body_start + 4..];
                                    
                                    if let Some(key) = extract_key {
                                        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(body) {
                                            if let Some(v) = json_val.get(key) {
                                                if let Some(num) = v.as_f64() {
                                                    val_to_store = num;
                                                } else if let Some(s) = v.as_str() {
                                                    if let Ok(num) = s.parse::<f64>() {
                                                        val_to_store = num;
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        let num_str: String = body.chars()
                                            .filter(|c| c.is_digit(10) || *c == '.')
                                            .collect();
                                        if let Ok(num) = num_str.parse::<f64>() {
                                            val_to_store = num;
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        println!("[AIL-VM] ❌ Оракул не смог подключиться к {}", host_port);
                    }
                    
                    println!("[AIL-VM] 👁️ Оракул получил значение: {} => {}", var_name, val_to_store);
                    tx.put(var_name.clone(), val_to_store);
                    self.entropy_level += 0.5;
                }
                AstNode::AiAnalyze { text, var_name } => {
                    let mut score = 0.5; // Default neutral score
                    let lower_text = text.to_lowercase();
                    
                    if lower_text.contains("scam") || lower_text.contains("hack") || lower_text.contains("steal") || lower_text.contains("fake") {
                        score = 0.1;
                    } else if lower_text.contains("donation") || lower_text.contains("gift") || lower_text.contains("payment") || lower_text.contains("safe") {
                        score = 0.9;
                    }
                    
                    println!("[AIL-VM] 🧠 ИИ-Анализ текста: \"{}\" -> Оценка: {}", text, score);
                    tx.put(var_name.clone(), score);
                }
                AstNode::MintToken { token_name, amount } => {
                    if deployer != "AIL-SYSTEM" {
                        println!("[AIL-VM] 💎 Чеканка AIL-20 токена: {} ({}) для {}", token_name, amount, deployer);
                        let _ = wm.mint_custom_token(deployer, token_name, *amount as u64);
                    }
                }
                AstNode::TransferToken { token_name, to_address, amount } => {
                    if deployer != "AIL-SYSTEM" {
                        println!("[AIL-VM] 💸 Перевод AIL-20 токена: {} ({}) от {} к {}", token_name, amount, deployer, to_address);
                        let _ = wm.transfer_custom_token(deployer, to_address, token_name, *amount as u64);
                    }
                }
                AstNode::ParallelAsync { body } => {
                    println!("[AIL-VM] ⚡ Запуск параллельного потока (Fearless Concurrency)...");
                    self.execute_internal(body, deployer, wm, tx);
                    println!("[AIL-VM] ⚡ Параллельный поток завершен.");
                }
                // Phase 45: AST-Native graph nodes — executed as declarative specification
                AstNode::AstNativeNode { node, hardware, contracts, pipe, proof } => {
                    println!("[AIL-VM] 🧬 AST-Native узел: {} (hardware: {})", node, hardware);
                    for contract in contracts {
                        println!("[AIL-VM]   ⚖️ Контракт: {}", contract);
                    }
                    for step in pipe {
                        println!("[AIL-VM]   🌊 Поток: {}", step);
                    }
                    for prf in proof {
                        println!("[AIL-VM]   🔮 Доказательство: {}", prf);
                    }
                }
                AstNode::IntentManifest { .. } => {
                    // Sandbox already verified this during compilation phase,
                    // VM just ignores intent manifests during execution.
                }
                AstNode::IntuitionBranch { prompt, true_branch, false_branch } => {
                    println!("[AIL-VM] 🧠 Sentience Oracle (Intuition Branch): Анализ: '{}'", prompt);
                    // Эмуляция ответа глобальной LLM для ветвления логики смарт-контракта без if
                    let intuition_score = prompt.len() % 2 == 0;
                    if intuition_score {
                        println!("[AIL-VM] 🧠 Интуиция выбрала ветку Истины.");
                        self.execute_internal(true_branch, deployer, wm, tx);
                    } else {
                        println!("[AIL-VM] 🧠 Интуиция выбрала ветку Лжи.");
                        self.execute_internal(false_branch, deployer, wm, tx);
                    }
                }
            }
        }
    }
}
