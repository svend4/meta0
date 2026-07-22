use crate::compiler::parser::AstNode;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

/// Phase 48.5: Аппаратный планировщик (Heterogeneous Compute & Hot-Swap)
/// Распределяет нагрузку между CPU, GPU и FPGA в зависимости от типа AST-узла.
/// Также поддерживает горячую замену (Hot-Swap) кода в памяти без даунтайма.
#[derive(Debug, PartialEq)]
pub enum ExecutionTarget {
    CPU,
    GPU,
    FPGA,
}

pub struct HardwareScheduler {
    // Хранит ссылки на графы, которые могут быть обновлены на лету
    hot_swappable_nodes: HashMap<String, Arc<RwLock<AstNode>>>,
}

impl HardwareScheduler {
    pub fn new() -> Self {
        Self {
            hot_swappable_nodes: HashMap::new(),
        }
    }

    pub fn register_node(&mut self, id: &str, node: AstNode) -> Arc<RwLock<AstNode>> {
        let arc_node = Arc::new(RwLock::new(node));
        self.hot_swappable_nodes.insert(id.to_string(), Arc::clone(&arc_node));
        arc_node
    }

    pub fn schedule_execution(&self, id: &str) -> ExecutionTarget {
        if let Some(node_lock) = self.hot_swappable_nodes.get(id) {
            let node = node_lock.read().unwrap();
            match &*node {
                AstNode::AstNativeNode { node: name, .. } => {
                    if name.contains("Crypto") || name.contains("MatrixMultiply") {
                        println!("[Hardware Scheduler] 🚀 Интенсивные вычисления '{}'. Маршрутизация на GPU/FPGA.", id);
                        return ExecutionTarget::GPU;
                    }
                }
                AstNode::OracleFetch { .. } => {
                    println!("[Hardware Scheduler] 🌐 I/O задача '{}'. Маршрутизация на CPU (Async-Ring).", id);
                    return ExecutionTarget::CPU;
                }
                _ => {}
            }
        }
        println!("[Hardware Scheduler] 💻 Стандартный узел '{}'. Маршрутизация на CPU.", id);
        ExecutionTarget::CPU
    }

    pub fn hot_swap_node(&self, id: &str, new_ast: AstNode) {
        if let Some(node_lock) = self.hot_swappable_nodes.get(id) {
            let mut node = node_lock.write().unwrap();
            *node = new_ast;
            println!("[Hardware Scheduler] 🔥 HOT-SWAP: Код узла '{}' обновлен в памяти без остановки (Zero-Downtime)!", id);
        }
    }
}
