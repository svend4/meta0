// ============================================================
// AIL Проход 3 — Честный Self-Healing Runtime (Verified Hot-Swap)
//
// Идея из Google-Search_2026_07_22__0812.md (супер-ядра v3/v4): при аномалии
// рантайм синтезирует патч узла и горячо подменяет логику. У лога было два
// фатальных дефекта, которые здесь исправлены принципиально:
//
//  1. UB-механика: AtomicPtr::swap + немедленный Box::from_raw = use-after-free
//     под конкуренцией. Здесь — RwLock<Arc<ManagedPipeline>>: читатели держат
//     свой Arc-снапшот, своп атомарен, старая версия умирает, когда её
//     отпустит последний читатель. Ноль unsafe.
//
//  2. Деградация контрактов: «ремонт» в логе ОСЛАБЛЯЛ инвариант (порог 500→100),
//     то есть система чинила отказ, переписывая правило, которое отказ вызвала.
//     Здесь патч обязан пройти через настоящий FormalVerifier И через проверку
//     сохранения контрактов: каждый pre-контракт активной версии должен
//     присутствовать в кандидате с НЕ более слабой границей; лимит памяти
//     нельзя расширять. Ослабление = отказ в свопе.
// ============================================================

use crate::compiler::lexer::Lexer;
use crate::compiler::parser::{AstNode, Parser};
use crate::compiler::verifier::{FormalVerifier, VerificationStatus};
use std::sync::{Arc, Mutex, RwLock};

/// Плейсхолдер суммы транзакции в шаблоне узла.
pub const AMOUNT_PLACEHOLDER: &str = "{AMOUNT}";

#[derive(Debug, Clone)]
pub struct PipelineMeta {
    pub node_id: String,
    pub schema_version: u32,
}

/// Управляемый узел: метаданные + исходник-шаблон + извлечённые контракты.
pub struct ManagedPipeline {
    pub meta: PipelineMeta,
    pub source_template: String,
    /// (переменная, оператор, граница) из [contract::pre(...)]
    pub pre_contracts: Vec<(String, String, f64)>,
    /// Лимит из [contract::max_allocation(N, bytes)], если есть
    pub max_allocation: Option<f64>,
}

/// Запись аудита самолечения (уходит в EventStore/mempool на уровне ноды).
#[derive(Debug, Clone)]
pub struct HealingAuditEntry {
    pub node_id: String,
    pub from_version: u32,
    pub to_version: u32,
    pub verdict: String,
}

pub struct SelfHealingRegistry {
    active: RwLock<Arc<ManagedPipeline>>,
    pub audit_log: Mutex<Vec<HealingAuditEntry>>,
}

// ── Вспомогательное: компиляция шаблона и извлечение контрактов ──────────

fn compile_template(node_id: &str, template: &str, probe_amount: f64) -> Result<(Vec<AstNode>, Vec<(String, String, f64)>, Option<f64>), String> {
    let source = template.replace(AMOUNT_PLACEHOLDER, &format!("{}", probe_amount));
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse();
    if ast.is_empty() {
        return Err("пустой AST: шаблон не распарсился".to_string());
    }

    let mut verifier = FormalVerifier::new();
    let report = verifier.verify_ast_full(node_id, &ast);
    if report.status != VerificationStatus::Passed {
        let reason = report
            .errors
            .first()
            .map(|e| e.reason.clone())
            .unwrap_or_else(|| "verification failed".to_string());
        return Err(format!("верификатор отклонил шаблон: {}", reason));
    }

    let (pre, max_alloc) = extract_contracts(&ast);
    Ok((ast, pre, max_alloc))
}

fn extract_contracts(ast: &[AstNode]) -> (Vec<(String, String, f64)>, Option<f64>) {
    let mut pre = Vec::new();
    let mut max_alloc = None;
    collect(ast, &mut pre, &mut max_alloc);
    return (pre, max_alloc);

    fn collect(nodes: &[AstNode], pre: &mut Vec<(String, String, f64)>, max_alloc: &mut Option<f64>) {
        for n in nodes {
            match n {
                AstNode::ContractPre { var_name, operator, limit } => {
                    pre.push((var_name.clone(), operator.clone(), *limit));
                }
                AstNode::ContractMaxAllocation { bytes } => {
                    *max_alloc = Some(match max_alloc {
                        Some(prev) => prev.min(*bytes),
                        None => *bytes,
                    });
                }
                AstNode::IfCondition { body, .. }
                | AstNode::Loop { body, .. }
                | AstNode::ParallelAsync { body } => collect(body, pre, max_alloc),
                _ => {}
            }
        }
    }
}

/// Ядро Прохода 3: правило «патч не может ослаблять контракты».
/// Для каждого pre-контракта активной версии ищем контракт кандидата по той же
/// переменной и оператору и требуем НЕ более слабую границу:
///   >= / >  : граница кандидата должна быть >= активной
///   <= / <  : граница кандидата должна быть <= активной
///   == / != : граница должна совпадать
/// Отсутствие контракта в кандидате = ослабление = отказ.
fn contracts_not_weakened(
    active_pre: &[(String, String, f64)],
    active_alloc: Option<f64>,
    candidate_pre: &[(String, String, f64)],
    candidate_alloc: Option<f64>,
) -> Result<(), String> {
    for (var, op, limit) in active_pre {
        let candidate = candidate_pre
            .iter()
            .find(|(v, o, _)| v == var && o == op);
        match candidate {
            None => {
                return Err(format!(
                    "патч удаляет контракт [contract::pre({} {} {})] — ослабление запрещено",
                    var, op, limit
                ));
            }
            Some((_, _, cand_limit)) => {
                let ok = match op.as_str() {
                    ">" | ">=" => cand_limit >= limit,
                    "<" | "<=" => cand_limit <= limit,
                    "==" | "!=" => (cand_limit - limit).abs() < f64::EPSILON,
                    _ => true,
                };
                if !ok {
                    return Err(format!(
                        "патч ослабляет контракт: было pre({} {} {}), стало pre({} {} {})",
                        var, op, limit, var, op, cand_limit
                    ));
                }
            }
        }
    }

    // Лимит памяти: расширять нельзя, сужать/добавлять можно.
    if let Some(active_bytes) = active_alloc {
        match candidate_alloc {
            None => {
                return Err(format!(
                    "патч удаляет [contract::max_allocation({}, bytes)] — ослабление запрещено",
                    active_bytes
                ));
            }
            Some(cand_bytes) if cand_bytes > active_bytes => {
                return Err(format!(
                    "патч расширяет лимит памяти: {} → {} bytes — ослабление запрещено",
                    active_bytes, cand_bytes
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

impl SelfHealingRegistry {
    /// Создать реестр с начальной версией узла (schema v100).
    /// Начальный шаблон тоже проходит верификацию — «грязный» узел не взлетит.
    pub fn new(node_id: &str, source_template: &str) -> Result<Self, String> {
        let (_ast, pre, alloc) = compile_template(node_id, source_template, 0.0)?;
        let pipeline = ManagedPipeline {
            meta: PipelineMeta { node_id: node_id.to_string(), schema_version: 100 },
            source_template: source_template.to_string(),
            pre_contracts: pre,
            max_allocation: alloc,
        };
        Ok(SelfHealingRegistry {
            active: RwLock::new(Arc::new(pipeline)),
            audit_log: Mutex::new(Vec::new()),
        })
    }

    /// Текущий активный узел (Arc-снапшот; живёт у читателя сколько нужно).
    pub fn current(&self) -> Arc<ManagedPipeline> {
        Arc::clone(&self.active.read().unwrap())
    }

    pub fn current_version(&self) -> u32 {
        self.active.read().unwrap().meta.schema_version
    }

    /// Исполнить транзакцию через активный узел: подставляем сумму в шаблон и
    /// прогоняем через верификатор (pre-flight proof). Err = аномалия:
    /// контракт узла отклонил операцию ДО каких-либо мутаций.
    pub fn execute_transaction(&self, amount: f64) -> Result<(), String> {
        let pipeline = self.current();
        match compile_template(&pipeline.meta.node_id, &pipeline.source_template, amount) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Попытка горячей замены узла. Возвращает новую версию схемы или причину отказа.
    /// Порядок обороны:
    ///   1) кандидат компилируется и проходит FormalVerifier (Passed);
    ///   2) кандидат не ослабляет ни один контракт активной версии;
    ///   3) только после этого — атомарный своп (RwLock write) + запись в аудит.
    pub fn try_hot_swap(&self, candidate_template: &str) -> Result<u32, String> {
        let active = self.current();
        let node_id = active.meta.node_id.clone();

        // Шаг 1: формальная верификация кандидата.
        let (_ast, cand_pre, cand_alloc) = compile_template(&node_id, candidate_template, 0.0)
            .map_err(|e| {
                self.audit(&node_id, active.meta.schema_version, active.meta.schema_version,
                    &format!("REJECTED (verifier): {}", e));
                format!("REJECTED: {}", e)
            })?;

        // Шаги 2-3 под write-lock: сравниваем контракты с АКТУАЛЬНОЙ версией
        // (не со снапшотом — иначе TOCTOU при конкурирующих свопах) и свопаем.
        let mut guard = self.active.write().unwrap();
        if let Err(e) = contracts_not_weakened(
            &guard.pre_contracts,
            guard.max_allocation,
            &cand_pre,
            cand_alloc,
        ) {
            let v = guard.meta.schema_version;
            drop(guard);
            self.audit(&node_id, v, v, &format!("REJECTED (contract-preservation): {}", e));
            return Err(format!("REJECTED: {}", e));
        }
        let new_version = guard.meta.schema_version + 1;
        let new_pipeline = ManagedPipeline {
            meta: PipelineMeta { node_id: node_id.clone(), schema_version: new_version },
            source_template: candidate_template.to_string(),
            pre_contracts: cand_pre,
            max_allocation: cand_alloc,
        };
        let old_version = guard.meta.schema_version;
        *guard = Arc::new(new_pipeline);
        drop(guard);

        self.audit(&node_id, old_version, new_version, "ACCEPTED (verified hot-swap)");
        println!(
            "[SelfHealing] ✅ Узел {} горячо заменён: schema v{} → v{} (патч доказан верификатором)",
            node_id, old_version, new_version
        );
        Ok(new_version)
    }

    fn audit(&self, node_id: &str, from: u32, to: u32, verdict: &str) {
        let entry = HealingAuditEntry {
            node_id: node_id.to_string(),
            from_version: from,
            to_version: to,
            verdict: verdict.to_string(),
        };
        println!("[SelfHealing] 📋 Аудит: {} v{}→v{}: {}", entry.node_id, from, to, verdict);
        self.audit_log.lock().unwrap().push(entry);
    }
}

/// Демо-шаблон узла-стража транзакций (v100):
/// резерв после списания не должен падать ниже 500.
pub fn demo_guard_template_v100() -> String {
    [
        "MODULE TxGuard",
        "[contract::pre(balance >= 500)]",
        "[contract::max_allocation(1000, bytes)]",
        "STORE balance => 1000",
        &format!("SUB balance => {}", AMOUNT_PLACEHOLDER),
    ]
    .join("\n")
}

/// «Патч из лога»: ослабляет инвариант 500 → 100. Обязан быть отклонён.
pub fn demo_weakening_patch() -> String {
    demo_guard_template_v100().replace("balance >= 500", "balance >= 100")
}

/// Честный патч: контракт сохранён и УСИЛЕН (500 → 600). Обязан быть принят.
pub fn demo_strengthening_patch() -> String {
    demo_guard_template_v100().replace("balance >= 500", "balance >= 600")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn initial_pipeline_compiles_and_extracts_contracts() {
        let reg = SelfHealingRegistry::new("TX_GUARD", &demo_guard_template_v100()).unwrap();
        let p = reg.current();
        assert_eq!(p.meta.schema_version, 100);
        assert_eq!(p.pre_contracts.len(), 1);
        assert_eq!(p.pre_contracts[0].0, "balance");
        assert_eq!(p.pre_contracts[0].2, 500.0);
        assert_eq!(p.max_allocation, Some(1000.0));
    }

    #[test]
    fn transaction_guard_blocks_anomaly() {
        let reg = SelfHealingRegistry::new("TX_GUARD", &demo_guard_template_v100()).unwrap();
        // 1000 - 200 = 800 >= 500 → штатно
        assert!(reg.execute_transaction(200.0).is_ok());
        // 1000 - 600 = 400 < 500 → аномалия, контракт держит удар
        assert!(reg.execute_transaction(600.0).is_err());
    }

    #[test]
    fn weakening_patch_rejected() {
        let reg = SelfHealingRegistry::new("TX_GUARD", &demo_guard_template_v100()).unwrap();
        let err = reg.try_hot_swap(&demo_weakening_patch()).unwrap_err();
        assert!(err.contains("ослабляет"), "ожидали отказ по ослаблению, получили: {}", err);
        assert_eq!(reg.current_version(), 100, "версия не должна меняться при отказе");
    }

    #[test]
    fn strengthening_patch_accepted() {
        let reg = SelfHealingRegistry::new("TX_GUARD", &demo_guard_template_v100()).unwrap();
        let v = reg.try_hot_swap(&demo_strengthening_patch()).unwrap();
        assert_eq!(v, 101);
        // Теперь транзакция 450 (резерв 550 < 600) — аномалия по усиленному контракту.
        assert!(reg.execute_transaction(450.0).is_err());
        // А 300 (резерв 700 >= 600) — штатно.
        assert!(reg.execute_transaction(300.0).is_ok());
    }

    #[test]
    fn contract_removal_rejected() {
        let reg = SelfHealingRegistry::new("TX_GUARD", &demo_guard_template_v100()).unwrap();
        // Патч без pre-контракта вовсе
        let candidate = [
            "MODULE TxGuard",
            "[contract::max_allocation(1000, bytes)]",
            "STORE balance => 1000",
            &format!("SUB balance => {}", AMOUNT_PLACEHOLDER),
        ]
        .join("\n");
        let err = reg.try_hot_swap(&candidate).unwrap_err();
        assert!(err.contains("удаляет контракт"));
    }

    #[test]
    fn memory_limit_expansion_rejected() {
        let reg = SelfHealingRegistry::new("TX_GUARD", &demo_guard_template_v100()).unwrap();
        let candidate = demo_guard_template_v100().replace("max_allocation(1000", "max_allocation(50000");
        let err = reg.try_hot_swap(&candidate).unwrap_err();
        assert!(err.contains("расширяет лимит памяти"));
    }

    #[test]
    fn broken_patch_rejected_by_verifier() {
        let reg = SelfHealingRegistry::new("TX_GUARD", &demo_guard_template_v100()).unwrap();
        // Кандидат с делением на ноль — верификатор обязан поймать до свопа.
        let candidate = format!("{}\nDIV balance => 0", demo_guard_template_v100());
        let err = reg.try_hot_swap(&candidate).unwrap_err();
        assert!(err.contains("REJECTED"), "err: {}", err);
        assert_eq!(reg.current_version(), 100);
    }

    // Конкурентная безопасность свопа: читатели исполняют транзакции,
    // пока другой поток свопает версию. Раньше (AtomicPtr + Box::from_raw)
    // это был бы use-after-free; с RwLock<Arc> — ни одного unsafe.
    #[test]
    fn concurrent_readers_survive_hot_swap() {
        let reg = std::sync::Arc::new(
            SelfHealingRegistry::new("TX_GUARD", &demo_guard_template_v100()).unwrap(),
        );

        let mut readers = Vec::new();
        for _ in 0..4 {
            let r = std::sync::Arc::clone(&reg);
            readers.push(thread::spawn(move || {
                for _ in 0..20 {
                    // Не паникуем ни при каком исходе — важно отсутствие UB/крэша.
                    let _ = r.execute_transaction(200.0);
                }
            }));
        }
        let swapper = {
            let r = std::sync::Arc::clone(&reg);
            thread::spawn(move || {
                let _ = r.try_hot_swap(&demo_strengthening_patch());
            })
        };
        for h in readers {
            h.join().unwrap();
        }
        swapper.join().unwrap();
        assert!(reg.current_version() >= 100);
    }
}
