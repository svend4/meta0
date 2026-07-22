use crate::compiler::parser::AstNode;
use crate::compiler::vm::AilVirtualMachine;
use crate::ledger::AilLedger;
use crate::wallet::WalletManager;
use crate::state_tree::AilStateTree;

pub struct SmartContract {
    contract_id: String,
    deployer: String,
    ast: Vec<AstNode>,
}

impl SmartContract {
    pub fn new(contract_id: &str, deployer: &str, ast: Vec<AstNode>) -> Self {
        println!("[SmartContract] 📝 Создан смарт-контракт: {} от {}", contract_id, deployer);
        SmartContract {
            contract_id: contract_id.to_string(),
            deployer: deployer.to_string(),
            ast,
        }
    }

    pub fn execute_and_commit(&self, vm: &mut AilVirtualMachine, ledger: &mut AilLedger, wm: &mut WalletManager, state_tree: &mut AilStateTree) {
        println!("[SmartContract] ⚖️ Исполнение контракта {} на AIL-VM...", self.contract_id);
        
        match vm.execute(self.ast.clone(), &self.deployer, wm, state_tree) {
            Ok(root_hash) => {
                let state_data = format!("CONTRACT_EXECUTED: {} | UNIFIED_STATE_ROOT: {}", self.contract_id, root_hash);
                ledger.add_block(state_data);
                
                // JIT Codegen Phase 39
                let codegen = crate::compiler::codegen::AilCodegen::new();
                let generated_rust = codegen.generate_rust(&self.ast, &self.contract_id);
                codegen.save_generated_code(&generated_rust, &self.contract_id);
            }
            Err(e) => {
                println!("[SmartContract] ❌ Контракт отклонен. Откат транзакции. Причина: {}", e);
                // No ledger entry on abort, state remains clean
            }
        }
    }
}
