import json

# 1. СЫРОЙ AST-ГРАФ ПРОГРАММЫ (Эквивалент бинарного файла AIL будущего)
ast_json_data = """
{
  "ast_root": {
    "node_id": "ND_001",
    "metadata": { "module": "wallet_core", "execution_policy": "pure_stateless" },
    "body_graph": {
      "nodes": {
        "OP_BindState": { "operation": "SYS_STATE_BIND", "target_store": "GlobalLedger" },
        "OP_GetBalance": { "operation": "MAP_GET_PROPERTY", "property": "balance" },
        "OP_CheckInvariant": { "operation": "MATH_SUB_SAFE_PROOF" },
        "BRANCH_Decision": {
          "operation": "CONDITIONAL_BRANCH",
          "branches": {
            "on_success": { "operation": "STATE_MUTATE_SUB", "property": "balance", "exit_code": 200 },
            "on_overflow": { "operation": "FORWARD_EXIT", "exit_code": 402 }
          }
        }
      }
    }
  }
}
"""

# 2. ДВИЖОК ВИРТУАЛЬНОЙ МАШИНЫ АИЛ (AIL Runtime Engine)
class AILVirtualMachine:
    def __init__(self, global_state_space):
        # Имитируем Единое Распределенное Пространство Состояний (вместо внешних БД)
        self.state_space = global_state_space

    def execute_node(self, ast_tree, runtime_inputs):
        """Интерпретирует связи и узлы AST-графа БЕЗ текстового парсинга кода"""
        root = ast_tree["ast_root"]
        nodes = root["body_graph"]["nodes"]

        # Локальный контекст исполнения внутри регистров ВМ
        ctx = {
            "wallet_id": runtime_inputs.get("wallet_id"),
            "input_amount": runtime_inputs.get("input_amount"),
            "current_balance": 0,
            "bound_wallet_ref": None
        }

        # Шаг 1: Исполнение узла SYS_STATE_BIND (Привязка к памяти БД)
        target_store = nodes["OP_BindState"]["target_store"]
        if target_store in self.state_space and ctx["wallet_id"] in self.state_space[target_store]:
            ctx["bound_wallet_ref"] = self.state_space[target_store][ctx["wallet_id"]]
        else:
            return 404, "Runtime Error: Wallet node context address not found"

        # Шаг 2: Исполнение узла MAP_GET_PROPERTY (Извлечение баланса)
        prop_name = nodes["OP_GetBalance"]["property"]
        ctx["current_balance"] = ctx["bound_wallet_ref"][prop_name]

        # Шаг 3: Математическая верификация инварианта на лету (MATH_SUB_SAFE_PROOF)
        # Проверяем закон: баланс не может уйти ниже запрашиваемой суммы
        invariant_proven = ctx["current_balance"] >= ctx["input_amount"]

        # Шаг 4: Аппаратное ветвление (CONDITIONAL_BRANCH)
        branches = nodes["BRANCH_Decision"]["branches"]

        if invariant_proven:
            # Математически безопасно: мутируем состояние напрямую (Zero-Copy Mutation)
            success_action = branches["on_success"]
            target_prop = success_action["property"]

            ctx["bound_wallet_ref"][target_prop] -= ctx["input_amount"] # Прямое списание
            return success_action["exit_code"], f"Executed. New balance: {ctx['bound_wallet_ref'][target_prop]}"
        else:
            # Риск нарушения инварианта: прерываем транзакцию
            overflow_action = branches["on_overflow"]
            return overflow_action["exit_code"], "Rejected: Transaction violates safety invariant (Insufficient funds)"

# ==========================================
# ПОДГОТОВКА И ТЕСТИРОВАНИЕ ПРОТОТИПА
# ==========================================

# Создаем симуляцию общей памяти (кошелек с балансом 1000 единиц)
mock_distributed_memory = {
    "GlobalLedger": {
        "wallet-user-777": {"balance": 1000}
    }
}

# Инициализируем Виртуальную Машину
ail_vm = AILVirtualMachine(mock_distributed_memory)
parsed_ast = json.loads(ast_json_data)

print("--- ЗАПУСК ПРОТОТИПА СРЕДЫ AIL (ИЮЛЬ 2026) ---\n")

# Симуляция Запроса 1: Списание 400 единиц (Безопасно)
print("Запрос 1: Попытка списания 400 единиц...")
code_1, msg_1 = ail_vm.execute_node(parsed_ast, {"wallet_id": "wallet-user-777", "input_amount": 400})
print(f"Ответ ВМ: Код [{code_1}] -> {msg_1}\n")

# Симуляция Запроса 2: Попытка списать 700 единиц (Остаток 600 -> Опасность овердрафта!)
print("Запрос 2: Попытка списания 700 единиц (при балансе 600)...")
code_2, msg_2 = ail_vm.execute_node(parsed_ast, {"wallet_id": "wallet-user-777", "input_amount": 700})
print(f"Ответ ВМ: Код [{code_2}] -> {msg_2}\n")