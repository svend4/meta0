use crate::compiler::parser::AstNode;
use crate::compiler::vm::AilVirtualMachine;
use crate::ledger::AilLedger;

pub struct SmartContract {
    contract_id: String,
    ast: Vec<AstNode>,
}

impl SmartContract {
    pub fn new(contract_id: &str, ast: Vec<AstNode>) -> Self {
        println!("[SmartContract] 📝 Создан смарт-контракт: {}", contract_id);
        SmartContract {
            contract_id: contract_id.to_string(),
            ast,
        }
    }

    pub fn execute_and_commit(&self, vm: &mut AilVirtualMachine, ledger: &mut AilLedger) {
        println!("[SmartContract] ⚖️ Исполнение контракта {} на AIL-VM...", self.contract_id);
        vm.execute(self.ast.clone());
        
        let state_data = format!("CONTRACT_EXECUTED: {}", self.contract_id);
        ledger.add_block(state_data);
    }
}
