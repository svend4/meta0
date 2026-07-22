// ============================================================
// AIL Проход 4 — Spec Loader: делает .ail-спеки применимыми
//
// В ail_specs/ лежат 14 файлов на «настоящем» декларативном AIL
// (@module::, [contract::...], state::Name {}, node::Name() -> () {},
// stream::pipe/mutate, proof::invariant/assert, type:: enum/struct).
// Раньше они были мёртвым корпусом: parse_ast_native ждёт строки вида
// «node::»/«contract::» без скобок/сигнатур и эти файлы не разбирает.
//
// Этот модуль даёт им ПРИМЕНЕНИЕ:
//   1) структурный разбор реальной грамматики .ail в AilSpec;
//   2) извлечение машинно-проверяемых контрактов ([contract::pre(x op N)],
//      [contract::max_allocation(N, bytes)]) и мутаций (x += N / x -= N из
//      stream::mutate) в AstNode → прогон через настоящий FormalVerifier;
//   3) структурированный отчёт (для CLI/ноды/CI).
//
// Это не полный компилятор AIL (язык богаче императивного DSL ноды) —
// это честный «reader + verifier surface»: что машинно-проверяемо, то
// проверяется; что декларативно — то инвентаризуется.
// ============================================================

use crate::compiler::parser::AstNode;
use crate::compiler::verifier::{FormalVerifier, VerificationStatus};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StateField {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateBlock {
    pub name: String,
    pub annotations: Vec<String>,
    pub fields: Vec<StateField>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecNode {
    pub name: String,
    pub signature: String,
    pub contracts: Vec<String>,
    pub pipe_steps: Vec<String>,
    pub proofs: Vec<String>,
    pub mutations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationSummary {
    pub status: String,
    pub theorems_proven: usize,
    pub theorems_failed: usize,
    pub checkable_contracts: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecReport {
    pub module: Option<String>,
    pub directives: Vec<(String, String)>,
    pub top_contracts: Vec<String>,
    pub states: Vec<StateBlock>,
    pub types: Vec<String>,
    pub nodes: Vec<SpecNode>,
    pub top_proofs: Vec<String>,
    pub verification: VerificationSummary,
}

impl SpecReport {
    pub fn human_summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "module: {}\n",
            self.module.as_deref().unwrap_or("(нет @module)")
        ));
        if !self.directives.is_empty() {
            s.push_str("директивы:\n");
            for (k, v) in &self.directives {
                s.push_str(&format!("  {} = {}\n", k, v));
            }
        }
        s.push_str(&format!(
            "state-блоков: {} | типов: {} | узлов: {} | top-контрактов: {}\n",
            self.states.len(),
            self.types.len(),
            self.nodes.len(),
            self.top_contracts.len()
        ));
        for n in &self.nodes {
            s.push_str(&format!(
                "  node::{}  контрактов={} pipe={} proof={} мутаций={}\n",
                n.name,
                n.contracts.len(),
                n.pipe_steps.len(),
                n.proofs.len(),
                n.mutations.len()
            ));
        }
        s.push_str(&format!(
            "верификация: {} | теорем доказано={} / провалено={} (машинно-проверяемых контрактов: {})\n",
            self.verification.status,
            self.verification.theorems_proven,
            self.verification.theorems_failed,
            self.verification.checkable_contracts
        ));
        for e in &self.verification.errors {
            s.push_str(&format!("  ⚠ {}\n", e));
        }
        s
    }
}

fn strip_comment(line: &str) -> &str {
    // '#' начинает комментарий (в .ail именно '#', не '//').
    match line.find('#') {
        Some(idx) => &line[..idx],
        None => line,
    }
}

/// Извлечь (var, op, limit) из выражения вида "amount > 0" или "token::len >= 4".
/// var может содержать '::' — нормализуем в '_'. Возвращаем None, если RHS не число.
fn parse_pre_expr(expr: &str) -> Option<(String, String, f64)> {
    let ops = [">=", "<=", "==", "!=", ">", "<"];
    for op in ops {
        if let Some(pos) = expr.find(op) {
            let lhs = expr[..pos].trim();
            let rhs = expr[pos + op.len()..].trim();
            // Берём первое «слово» справа (до пробела/&&/скобки).
            let rhs_num: String = rhs
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '_')
                .collect();
            let rhs_clean = rhs_num.replace('_', "");
            if let Ok(n) = rhs_clean.parse::<f64>() {
                if !lhs.is_empty() {
                    let var = lhs.replace("::", "_");
                    return Some((var, op.to_string(), n));
                }
            }
            return None;
        }
    }
    None
}

/// Извлечь мутацию "x += N" / "x -= N" (N — число) из тела stream::mutate.
fn parse_mutation(line: &str) -> Option<AstNode> {
    for (op, is_add) in [("+=", true), ("-=", false)] {
        if let Some(pos) = line.find(op) {
            let var = line[..pos].trim().replace("::", "_");
            let rhs = line[pos + op.len()..].trim().trim_end_matches(';').trim();
            let rhs_num: String = rhs.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(n) = rhs_num.parse::<f64>() {
                if !var.is_empty() {
                    return Some(if is_add {
                        AstNode::MathAdd(var, n)
                    } else {
                        AstNode::MathSub(var, n)
                    });
                }
            }
        }
    }
    None
}

/// Извлечь max_allocation(N, bytes) из строки контракта.
fn parse_max_alloc(contract: &str) -> Option<f64> {
    let key = "max_allocation(";
    let start = contract.find(key)? + key.len();
    let rest = &contract[start..];
    let num: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '_').collect();
    num.replace('_', "").parse::<f64>().ok()
}

/// Главная точка входа: разобрать исходник .ail и прогнать верификатор.
pub fn analyze_spec(source: &str) -> SpecReport {
    let mut module = None;
    let mut directives: Vec<(String, String)> = Vec::new();
    let mut top_contracts: Vec<String> = Vec::new();
    let mut states: Vec<StateBlock> = Vec::new();
    let mut types: Vec<String> = Vec::new();
    let mut nodes: Vec<SpecNode> = Vec::new();
    let mut top_proofs: Vec<String> = Vec::new();

    // Накопитель контрактов/аннотаций перед следующим node:: или state::.
    let mut pending: Vec<String> = Vec::new();

    // Машинно-проверяемый AST для верификатора.
    let mut checkable: Vec<AstNode> = Vec::new();
    let mut checkable_contracts = 0usize;

    #[derive(PartialEq)]
    enum Ctx {
        Top,
        State,
        Node,
        Mutate,
    }
    let mut ctx = Ctx::Top;
    let mut depth: i32 = 0;
    let mut cur_state: Option<StateBlock> = None;
    let mut cur_node: Option<SpecNode> = None;

    for raw_line in source.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        // Учёт скобочной глубины после обработки строки (для выхода из блоков).
        let opens = line.matches('{').count() as i32;
        let closes = line.matches('}').count() as i32;

        // ── Директивы модуля ──
        if let Some(rest) = line.strip_prefix("@") {
            if let Some((k, v)) = rest.split_once("::") {
                let key = format!("@{}", k);
                let val = v.trim().to_string();
                if k == "module" {
                    module = Some(val.clone());
                }
                directives.push((key, val));
            }
            continue;
        }

        // ── Аннотации/контракты в скобках ──
        if line.starts_with('[') {
            let inner = line.trim_start_matches('[').trim_end_matches(']').to_string();
            // Машинно-проверяемое: contract::pre(...) и contract::max_allocation(...)
            if let Some(pre) = inner.strip_prefix("contract::pre(") {
                let expr = pre.trim_end_matches(')');
                if let Some((var, op, lim)) = parse_pre_expr(expr) {
                    checkable.push(AstNode::ContractPre { var_name: var, operator: op, limit: lim });
                    checkable_contracts += 1;
                }
            }
            if inner.contains("max_allocation(") {
                if let Some(bytes) = parse_max_alloc(&inner) {
                    checkable.push(AstNode::ContractMaxAllocation { bytes });
                    checkable_contracts += 1;
                }
            }
            if ctx == Ctx::Node {
                if let Some(n) = cur_node.as_mut() {
                    n.contracts.push(inner);
                }
            } else {
                pending.push(inner);
            }
            continue;
        }

        // ── type:: объявления ──
        if line.starts_with("type::") {
            types.push(line.to_string());
            depth += opens - closes;
            continue;
        }

        // ── state:: блок ──
        if line.starts_with("state::") && ctx == Ctx::Top {
            let name = line
                .trim_start_matches("state::")
                .split(|c| c == ' ' || c == '{')
                .next()
                .unwrap_or("")
                .to_string();
            cur_state = Some(StateBlock {
                name,
                annotations: std::mem::take(&mut pending),
                fields: Vec::new(),
            });
            ctx = Ctx::State;
            depth += opens - closes;
            if closes > opens {
                // однострочный (редко) — закрыть сразу
                if let Some(s) = cur_state.take() {
                    states.push(s);
                }
                ctx = Ctx::Top;
            }
            continue;
        }

        // ── node:: объявление ──
        if line.starts_with("node::") && (ctx == Ctx::Top) {
            let sig = line.trim_start_matches("node::").trim_end_matches('{').trim().to_string();
            let name = sig.split(|c| c == '(' || c == ' ').next().unwrap_or("").to_string();
            let mut contracts = std::mem::take(&mut pending);
            // top_contracts — те, что не привязались к узлу (обычно все привязываются)
            let node_contracts = std::mem::take(&mut contracts);
            cur_node = Some(SpecNode {
                name,
                signature: sig,
                contracts: node_contracts,
                pipe_steps: Vec::new(),
                proofs: Vec::new(),
                mutations: Vec::new(),
            });
            ctx = Ctx::Node;
            depth += opens - closes;
            continue;
        }

        // ── Внутри state-блока: поля ──
        if ctx == Ctx::State {
            depth += opens - closes;
            if line == "}" || (closes > opens) {
                if let Some(s) = cur_state.take() {
                    states.push(s);
                }
                ctx = Ctx::Top;
                continue;
            }
            // поле: "name: Type," (или с [storage::...] аннотацией)
            let field_line = line.trim_end_matches(',');
            if let Some((n, t)) = field_line.split_once(':') {
                let name = n.trim();
                let ty = t.trim();
                if !name.is_empty() && !ty.is_empty() && !name.starts_with('[') {
                    if let Some(s) = cur_state.as_mut() {
                        s.fields.push(StateField { name: name.to_string(), ty: ty.to_string() });
                    }
                }
            }
            continue;
        }

        // ── Внутри node-тела ──
        if ctx == Ctx::Node || ctx == Ctx::Mutate {
            // Вложенный stream::mutate — извлекаем мутации.
            if line.starts_with("stream::mutate") {
                ctx = Ctx::Mutate;
                depth += opens - closes;
                continue;
            }
            if line.starts_with("proof::") {
                if let Some(n) = cur_node.as_mut() {
                    n.proofs.push(line.to_string());
                }
            } else if line.starts_with("stream::pipe") || line.starts_with("->") || line.contains("->") {
                if let Some(n) = cur_node.as_mut() {
                    n.pipe_steps.push(line.to_string());
                }
            }
            if ctx == Ctx::Mutate {
                if let Some(node) = parse_mutation(line) {
                    checkable.push(node.clone());
                    if let Some(n) = cur_node.as_mut() {
                        n.mutations.push(line.to_string());
                    }
                }
            }

            depth += opens - closes;
            // Выход из mutate-подблока
            if ctx == Ctx::Mutate && depth <= 1 {
                ctx = Ctx::Node;
            }
            // Выход из узла целиком
            if depth <= 0 {
                if let Some(n) = cur_node.take() {
                    nodes.push(n);
                }
                ctx = Ctx::Top;
                depth = 0;
            }
            continue;
        }

        // ── Верхнеуровневый proof ──
        if line.starts_with("proof::") {
            top_proofs.push(line.to_string());
            depth += opens - closes;
            continue;
        }

        depth += opens - closes;
    }

    // Незакрытые блоки — на всякий случай.
    if let Some(s) = cur_state.take() {
        states.push(s);
    }
    if let Some(n) = cur_node.take() {
        nodes.push(n);
    }
    // top_contracts = pending, не привязавшийся ни к чему.
    top_contracts.append(&mut pending);

    // ── Прогон верификатора по извлечённым контрактам и мутациям ──
    let mut verifier = FormalVerifier::new();
    let module_name = module.clone().unwrap_or_else(|| "spec".to_string());
    let report = verifier.verify_ast_full(&module_name, &checkable);
    let verification = VerificationSummary {
        status: format!("{:?}", report.status),
        theorems_proven: report.theorems_proven,
        theorems_failed: report.theorems_failed,
        checkable_contracts,
        errors: report.errors.iter().map(|e| e.reason.clone()).collect(),
    };
    let _ = VerificationStatus::Passed; // (тип используется в отчёте verifier)

    SpecReport {
        module,
        directives,
        top_contracts,
        states,
        types,
        nodes,
        top_proofs,
        verification,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUTH: &str = "@module::auth_core_091\n@hardware_target::hyper_threaded_vector_cpu\n\n[contract::pure]\n[contract::pre(token::len > 0)]\n[contract::post(result::type == bool && delta::memory == 0)]\n\nnode::VerifyToken(token: Vector[u8]) -> (status: u1) {\n    stream::pipe {\n        input(token)\n        -> lookup::pure_cache_map(global_ctx::node_shared_01)\n    }\n    proof::invariant(\n        assert::no_panic,\n        assert::max_latency_cycles(420)\n    )\n}";

    const LEDGER: &str = "@module::financial_ledger\n@state_model::event_sourced\n\nstate::UserWallet {\n    id: UUID,\n    balance: i128,\n    currency: String[3],\n    version: u64\n}\n\n[contract::pre(amount > 0)]\n[contract::transactional(isolation::serializable)]\nnode::DepositFunds(wallet_id: UUID, amount: i128) -> (success: u1) {\n    let wallet = state::UserWallet::bind(wallet_id);\n    proof::assert(wallet::balance + amount <= i128::MAX);\n    stream::mutate(wallet) {\n        wallet::balance += 100;\n        wallet::version += 1;\n    }\n    forward::exit(1)\n}";

    #[test]
    fn parses_auth_spec() {
        let r = analyze_spec(AUTH);
        assert_eq!(r.module.as_deref(), Some("auth_core_091"));
        assert!(r.directives.iter().any(|(k, _)| k == "@hardware_target"));
        assert_eq!(r.nodes.len(), 1);
        assert_eq!(r.nodes[0].name, "VerifyToken");
        assert!(r.nodes[0].proofs.iter().any(|p| p.contains("proof::invariant")));
        // pre(token::len > 0) — машинно-проверяемый контракт
        assert!(r.verification.checkable_contracts >= 1);
    }

    #[test]
    fn parses_ledger_spec_with_state_and_mutations() {
        let r = analyze_spec(LEDGER);
        assert_eq!(r.module.as_deref(), Some("financial_ledger"));
        assert_eq!(r.states.len(), 1);
        assert_eq!(r.states[0].name, "UserWallet");
        assert_eq!(r.states[0].fields.len(), 4, "должно быть 4 поля кошелька");
        assert!(r.states[0].fields.iter().any(|f| f.name == "balance" && f.ty == "i128"));
        assert_eq!(r.nodes.len(), 1);
        assert_eq!(r.nodes[0].name, "DepositFunds");
        // stream::mutate: balance += 100, version += 1 → 2 мутации
        assert_eq!(r.nodes[0].mutations.len(), 2, "должны извлечься 2 мутации");
    }

    #[test]
    fn empty_source_is_safe() {
        let r = analyze_spec("");
        assert!(r.module.is_none());
        assert_eq!(r.nodes.len(), 0);
    }
}
