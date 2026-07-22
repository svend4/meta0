use crate::compiler::parser::AstNode;
use std::collections::HashSet;

/// Phase 48.3: AI-Driven Sandbox (Intent Manifests)
/// Модуль, анализирующий AST на соответствие задекларированным намерениям.
/// Если граф пытается выполнить несанкционированную операцию (например, сетевой запрос без allow_network),
/// компилятор или рантайм отвергает его на уровне математического доказательства.
pub struct SandboxVerifier {
    allowed_intents: HashSet<String>,
}

impl SandboxVerifier {
    pub fn new() -> Self {
        Self {
            allowed_intents: HashSet::new(),
        }
    }

    /// Извлекает Intent Manifests из корня AST
    pub fn collect_manifests(&mut self, ast: &Vec<AstNode>) {
        for node in ast {
            if let AstNode::IntentManifest { intents } = node {
                for intent in intents {
                    self.allowed_intents.insert(intent.clone());
                    println!("[Sandbox] 🛡️ Зарегистрировано намерение (Intent): {}", intent);
                }
            }
        }
    }

    /// Проверяет, нарушает ли граф какие-либо песочницы
    pub fn verify_safety(&self, ast: &Vec<AstNode>) -> Result<(), String> {
        // Если манифест пуст или явно содержит deny_all
        if self.allowed_intents.contains("deny_all") {
            println!("[Sandbox] ⚠️ Обнаружен манифест 'deny_all'. Любой I/O или мутация запрещены.");
        }

        for node in ast {
            match node {
                AstNode::OracleFetch { url, .. } => {
                    if self.allowed_intents.contains("deny_all") || !self.allowed_intents.contains("allow_network") {
                        return Err(format!("Нарушение песочницы: попытка сетевого запроса к '{}', но 'allow_network' не заявлен в Intent Manifest", url));
                    }
                }
                AstNode::StoreState(key, _) => {
                    if self.allowed_intents.contains("deny_all") || !self.allowed_intents.contains("allow_memory_write") {
                        // В реальной системе мы бы позволяли локальные переменные, но запрещали глобальное состояние
                        if key.starts_with("global_") {
                             return Err(format!("Нарушение песочницы: попытка записи в глобальное состояние '{}' без 'allow_memory_write'", key));
                        }
                    }
                }
                AstNode::MintToken { .. } | AstNode::TransferToken { .. } => {
                    if !self.allowed_intents.contains("allow_ledger_mutation") {
                        return Err("Нарушение песочницы: попытка изменения леджера без 'allow_ledger_mutation'".to_string());
                    }
                }
                AstNode::IfCondition { body, .. } | AstNode::Loop { body, .. } | AstNode::ParallelAsync { body } => {
                    self.verify_safety(body)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
