use super::lexer::Token;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AstNode {
    ModuleDecl(String),
    // New AST‑Native node representing graph elements
    AstNativeNode { node: String, hardware: String, contracts: Vec<String>, pipe: Vec<String>, proof: Vec<String> },
    StateDefinition { name: String, state_type: String },
    QuantumTransition { from: String, to: String },
    StoreState(String, f64),
    MathAdd(String, f64),
    MathSub(String, f64),
    MathMul(String, f64),
    MathDiv(String, f64),
    IfCondition { condition_var: String, operator: String, threshold: f64, body: Vec<AstNode> },
    Loop { count_var: String, body: Vec<AstNode> },
    OracleFetch { url: String, extract_key: Option<String>, var_name: String },
    AiAnalyze { text: String, var_name: String },
    MintToken { token_name: String, amount: f64 },
    TransferToken { token_name: String, to_address: String, amount: f64 },
    ContractPre { var_name: String, operator: String, limit: f64 },
    ContractMaxAllocation { bytes: f64 },
    IntentManifest { intents: Vec<String> }, // [sandbox::allow(...)]
    ParallelAsync { body: Vec<AstNode> },
    IntuitionBranch { prompt: String, true_branch: Vec<AstNode>, false_branch: Vec<AstNode> },
    CrossChainTeleport { target_chain: String, remote_address: String, payload: String },
}

pub struct Parser {
    tokens: Vec<Token>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens }
    }

    pub fn parse(&mut self) -> Vec<AstNode> {
        println!("[AIL Parser] 🧬 Построение многомерного AST графа...");
        let mut i = 0;
        self.parse_block(&mut i, false)
    }

    // Parse AST‑Native source (graph representation)
    pub fn parse_ast_native(source: &str) -> Vec<AstNode> {
        println!("[AIL Parser] 🧬 Parsing AST‑Native graph source...");
        let mut nodes = Vec::new();
        let mut current_node: Option<(String, String, Vec<String>, Vec<String>, Vec<String>)> = None;
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            if let Some(rest) = trimmed.strip_prefix("node::") {
                // finish previous node if exists
                if let Some((node, hardware, contracts, pipe, proof)) = current_node.take() {
                    nodes.push(AstNode::AstNativeNode { node, hardware, contracts, pipe, proof });
                }
                current_node = Some((rest.to_string(), String::new(), Vec::new(), Vec::new(), Vec::new()));
                continue;
            }
            if let Some(ref mut cur) = current_node {
                if let Some(rest) = trimmed.strip_prefix("hardware::") {
                    cur.1 = rest.to_string();
                } else if let Some(rest) = trimmed.strip_prefix("contract::") {
                    cur.2.push(rest.to_string());
                } else if let Some(rest) = trimmed.strip_prefix("pipe::") {
                    cur.3.push(rest.to_string());
                } else if let Some(rest) = trimmed.strip_prefix("proof::") {
                    cur.4.push(rest.to_string());
                }
            }
        }
        // push last node
        if let Some((node, hardware, contracts, pipe, proof)) = current_node {
            nodes.push(AstNode::AstNativeNode { node, hardware, contracts, pipe, proof });
        }
        nodes
    }

    fn parse_block(&self, i: &mut usize, inside_block: bool) -> Vec<AstNode> {
        let mut ast = Vec::new();
        while *i < self.tokens.len() {
            match &self.tokens[*i] {
                Token::Punctuation(p) if *p == '}' && inside_block => {
                    *i += 1;
                    break;
                }
                Token::Punctuation(p) if *p == '[' => {
                    if let Some(Token::Identifier(contract_str)) = self.tokens.get(*i + 1) {
                        if contract_str == "contract" {
                            if let Some(Token::Punctuation(':')) = self.tokens.get(*i + 2) {
                                if let Some(Token::Punctuation(':')) = self.tokens.get(*i + 3) {
                                    if let Some(Token::Identifier(method)) = self.tokens.get(*i + 4) {
                                        if method == "pre" {
                                            if let Some(Token::Punctuation('(')) = self.tokens.get(*i + 5) {
                                                if let Some(Token::Identifier(var_name)) = self.tokens.get(*i + 6) {
                                                    if let Some(Token::Operator(op)) = self.tokens.get(*i + 7) {
                                                        if let Some(Token::Number(limit)) = self.tokens.get(*i + 8) {
                                                            if let Some(Token::Punctuation(')')) = self.tokens.get(*i + 9) {
                                                                if let Some(Token::Punctuation(']')) = self.tokens.get(*i + 10) {
                                                                    ast.push(AstNode::ContractPre { var_name: var_name.clone(), operator: op.clone(), limit: *limit });
                                                                    *i += 10;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else if method == "max_allocation" {
                                            if let Some(Token::Punctuation('(')) = self.tokens.get(*i + 5) {
                                                if let Some(Token::Number(bytes)) = self.tokens.get(*i + 6) {
                                                    if let Some(Token::Punctuation(',')) = self.tokens.get(*i + 7) {
                                                        if let Some(Token::Identifier(b)) = self.tokens.get(*i + 8) {
                                                            if b == "bytes" {
                                                                if let Some(Token::Punctuation(')')) = self.tokens.get(*i + 9) {
                                                                    if let Some(Token::Punctuation(']')) = self.tokens.get(*i + 10) {
                                                                        ast.push(AstNode::ContractMaxAllocation { bytes: *bytes });
                                                                        *i += 10;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "MODULE" => {
                    if let Some(Token::Identifier(name)) = self.tokens.get(*i + 1) {
                        ast.push(AstNode::ModuleDecl(name.clone()));
                        *i += 1;
                    }
                }
                Token::Keyword(kw) if kw == "QUANTUM_STATE" => {
                    // Проход 3: тот же фикс перерасхода индекса, что и у STORE/ADD.
                    if let Some(Token::Identifier(name)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Operator(op)) = self.tokens.get(*i + 2) {
                            if op == "=>" {
                                if let Some(Token::Identifier(state)) = self.tokens.get(*i + 3) {
                                    ast.push(AstNode::StateDefinition { name: name.clone(), state_type: "QUANTUM_STATE".to_string() });
                                    ast.push(AstNode::QuantumTransition { from: name.clone(), to: state.clone() });
                                    *i += 3;
                                }
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "STORE" => {
                    // Проход 3: убран лишний `*i += 1` — он проглатывал следующий
                    // токен (напр. SUB после STORE). Теперь как у SUB/MUL/DIV:
                    // *i += 3 внутри + *i += 1 в конце цикла = ровно 4 токена.
                    if let Some(Token::Identifier(key)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Operator(op)) = self.tokens.get(*i + 2) {
                            if op == "=>" {
                                if let Some(Token::Number(val)) = self.tokens.get(*i + 3) {
                                    ast.push(AstNode::StoreState(key.clone(), *val));
                                    *i += 3;
                                }
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "ADD" => {
                    if let Some(Token::Identifier(key)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Operator(op)) = self.tokens.get(*i + 2) {
                            if op == "=>" {
                                if let Some(Token::Number(val)) = self.tokens.get(*i + 3) {
                                    ast.push(AstNode::MathAdd(key.clone(), *val));
                                    *i += 3;
                                }
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "SUB" => {
                    if let Some(Token::Identifier(var_name)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Operator(op)) = self.tokens.get(*i + 2) {
                            if op == "=>" {
                                if let Some(Token::Number(val)) = self.tokens.get(*i + 3) {
                                    ast.push(AstNode::MathSub(var_name.clone(), *val));
                                    *i += 3;
                                }
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "MUL" => {
                    if let Some(Token::Identifier(var_name)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Operator(op)) = self.tokens.get(*i + 2) {
                            if op == "=>" {
                                if let Some(Token::Number(val)) = self.tokens.get(*i + 3) {
                                    ast.push(AstNode::MathMul(var_name.clone(), *val));
                                    *i += 3;
                                }
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "DIV" => {
                    if let Some(Token::Identifier(var_name)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Operator(op)) = self.tokens.get(*i + 2) {
                            if op == "=>" {
                                if let Some(Token::Number(val)) = self.tokens.get(*i + 3) {
                                    ast.push(AstNode::MathDiv(var_name.clone(), *val));
                                    *i += 3;
                                }
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "IF" => {
                    if let Some(Token::Identifier(condition_var)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Operator(operator)) = self.tokens.get(*i + 2) {
                            if let Some(Token::Number(threshold)) = self.tokens.get(*i + 3) {
                                if let Some(Token::Punctuation(brace)) = self.tokens.get(*i + 4) {
                                    if *brace == '{' {
                                        *i += 5;
                                        let body = self.parse_block(i, true);
                                        ast.push(AstNode::IfCondition { condition_var: condition_var.clone(), operator: operator.clone(), threshold: *threshold, body });
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "LOOP" => {
                    if let Some(Token::Identifier(count_var)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Punctuation(brace)) = self.tokens.get(*i + 2) {
                            if *brace == '{' {
                                *i += 3;
                                let body = self.parse_block(i, true);
                                ast.push(AstNode::Loop { count_var: count_var.clone(), body });
                                continue;
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "PARALLEL" => {
                    if let Some(Token::Punctuation(brace)) = self.tokens.get(*i + 1) {
                        if *brace == '{' {
                            *i += 2;
                            let body = self.parse_block(i, true);
                            ast.push(AstNode::ParallelAsync { body });
                            continue;
                        }
                    }
                }
                Token::Keyword(kw) if kw == "ORACLE_FETCH" => {
                    if let Some(Token::StringLiteral(url)) = self.tokens.get(*i + 1) {
                        let mut extract_key = None;
                        let mut var_name = "oracle_result".to_string();
                        
                        if let Some(Token::Keyword(ext)) = self.tokens.get(*i + 2) {
                            if ext == "EXTRACT" {
                                if let Some(Token::Identifier(key)) = self.tokens.get(*i + 3) {
                                    extract_key = Some(key.clone());
                                    if let Some(Token::Keyword(as_kw)) = self.tokens.get(*i + 4) {
                                        if as_kw == "AS" {
                                            if let Some(Token::Identifier(var)) = self.tokens.get(*i + 5) {
                                                var_name = var.clone();
                                                *i += 5;
                                            }
                                        }
                                    } else {
                                        *i += 3;
                                    }
                                }
                            }
                        }
                        
                        ast.push(AstNode::OracleFetch { url: url.clone(), extract_key, var_name });
                        *i += 1;
                    }
                }
                Token::Keyword(kw) if kw == "AI_ANALYZE" => {
                    if let Some(Token::StringLiteral(text)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Keyword(as_kw)) = self.tokens.get(*i + 2) {
                            if as_kw == "AS" {
                                if let Some(Token::Identifier(var_name)) = self.tokens.get(*i + 3) {
                                    ast.push(AstNode::AiAnalyze { text: text.clone(), var_name: var_name.clone() });
                                    *i += 3;
                                }
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "INTUITION_BRANCH" => {
                    if let Some(Token::StringLiteral(prompt)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Punctuation(brace)) = self.tokens.get(*i + 2) {
                            if *brace == '{' {
                                *i += 3;
                                let true_branch = self.parse_block(i, true);
                                // Проход 4: фикс off-by-one. parse_block уже съел '}'
                                // и продвинул i ЗА него, поэтому ELSE — на позиции *i,
                                // а не *i+1 (иначе ветка ELSE не находилась никогда).
                                let mut false_branch = Vec::new();
                                if let Some(Token::Keyword(else_kw)) = self.tokens.get(*i) {
                                    if else_kw == "ELSE" {
                                        if let Some(Token::Punctuation(else_brace)) = self.tokens.get(*i + 1) {
                                            if *else_brace == '{' {
                                                *i += 2;
                                                false_branch = self.parse_block(i, true);
                                            }
                                        }
                                    }
                                }
                                 ast.push(AstNode::IntuitionBranch { prompt: prompt.clone(), true_branch, false_branch });
                                continue;
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "TELEPORT" => {
                    if let Some(Token::StringLiteral(target)) = self.tokens.get(*i + 1) {
                        let mut remote_addr = "0x0".to_string();
                        let mut payload_str = "{}".to_string();
                        if let Some(Token::Keyword(to_kw)) = self.tokens.get(*i + 2) {
                            if to_kw == "TO" {
                                if let Some(Token::StringLiteral(addr)) = self.tokens.get(*i + 3) {
                                    remote_addr = addr.clone();
                                    if let Some(Token::Keyword(p_kw)) = self.tokens.get(*i + 4) {
                                        if p_kw == "PAYLOAD" {
                                            if let Some(Token::StringLiteral(pld)) = self.tokens.get(*i + 5) {
                                                payload_str = pld.clone();
                                                *i += 5;
                                            }
                                        } else {
                                            *i += 3;
                                        }
                                    } else {
                                        *i += 3;
                                    }
                                }
                            }
                        }
                        ast.push(AstNode::CrossChainTeleport {
                            target_chain: target.clone(),
                            remote_address: remote_addr,
                            payload: payload_str,
                        });
                        *i += 1;
                    }
                }
                Token::Keyword(kw) if kw == "MINT_TOKEN" => {
                    if let Some(Token::StringLiteral(token_name)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Operator(op)) = self.tokens.get(*i + 2) {
                            if op == "=>" {
                                if let Some(Token::Number(amount)) = self.tokens.get(*i + 3) {
                                    ast.push(AstNode::MintToken { token_name: token_name.clone(), amount: *amount });
                                    *i += 3;
                                }
                            }
                        }
                    } else if let Some(Token::Identifier(token_name)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Operator(op)) = self.tokens.get(*i + 2) {
                            if op == "=>" {
                                if let Some(Token::Number(amount)) = self.tokens.get(*i + 3) {
                                    ast.push(AstNode::MintToken { token_name: token_name.clone(), amount: *amount });
                                    *i += 3;
                                }
                            }
                        }
                    }
                }
                Token::Keyword(kw) if kw == "TRANSFER_TOKEN" => {
                    if let Some(Token::StringLiteral(token_name)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Keyword(to)) = self.tokens.get(*i + 2) {
                            if to == "TO" {
                                if let Some(Token::StringLiteral(to_address)) = self.tokens.get(*i + 3) {
                                    if let Some(Token::Operator(op)) = self.tokens.get(*i + 4) {
                                        if op == "=>" {
                                            if let Some(Token::Number(amount)) = self.tokens.get(*i + 5) {
                                                ast.push(AstNode::TransferToken { token_name: token_name.clone(), to_address: to_address.clone(), amount: *amount });
                                                *i += 5;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(Token::Identifier(token_name)) = self.tokens.get(*i + 1) {
                        if let Some(Token::Keyword(to)) = self.tokens.get(*i + 2) {
                            if to == "TO" {
                                if let Some(Token::StringLiteral(to_address)) = self.tokens.get(*i + 3) {
                                    if let Some(Token::Operator(op)) = self.tokens.get(*i + 4) {
                                        if op == "=>" {
                                            if let Some(Token::Number(amount)) = self.tokens.get(*i + 5) {
                                                ast.push(AstNode::TransferToken { token_name: token_name.clone(), to_address: to_address.clone(), amount: *amount });
                                                *i += 5;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            *i += 1;
        }
        ast
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::Lexer;

    fn parse(src: &str) -> Vec<AstNode> {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize();
        Parser::new(tokens).parse()
    }

    #[test]
    fn store_then_sub_both_parsed() {
        // Регресс на фикс перерасхода индекса: STORE больше не глотает SUB.
        let ast = parse("MODULE M\nSTORE balance => 1000\nSUB balance => 600");
        assert!(ast.iter().any(|n| matches!(n, AstNode::StoreState(k, v) if k == "balance" && *v == 1000.0)));
        assert!(ast.iter().any(|n| matches!(n, AstNode::MathSub(k, v) if k == "balance" && *v == 600.0)),
            "SUB после STORE должен парситься: {:?}", ast);
    }

    #[test]
    fn intuition_branch_with_else() {
        // Регресс на off-by-one: ветка ELSE должна находиться.
        let ast = parse("INTUITION_BRANCH \"safe?\" {\nSTORE ok => 1\n} ELSE {\nSTORE ok => 0\n}");
        let ib = ast.iter().find_map(|n| match n {
            AstNode::IntuitionBranch { true_branch, false_branch, .. } => Some((true_branch, false_branch)),
            _ => None,
        });
        let (t, f) = ib.expect("IntuitionBranch должен распарситься");
        assert_eq!(t.len(), 1, "true-ветка");
        assert_eq!(f.len(), 1, "false-ветка (ELSE) должна найтись, off-by-one починен");
    }
}
