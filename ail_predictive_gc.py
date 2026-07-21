import json

print("--- ЗАПУСК AIL ПРЕДИКТИВНОГО СБОРЩИКА МУСОРА (PREDICTIVE GC) ---\n")

# Эмуляция сырого AST Графа (AIL) до компиляции
ast_graph = {
    "nodes": [
        {"id": "op_1", "action": "CREATE_VAR", "name": "temp_buffer", "size_bytes": 1024},
        {"id": "op_2", "action": "MUTATE_VAR", "name": "temp_buffer"},
        {"id": "op_3", "action": "NETWORK_SEND", "data": "temp_buffer"},
        {"id": "op_4", "action": "COMPUTE_HASH", "data": "transaction_signature"}
    ],
    "edges": [
        ("op_1", "op_2"),
        ("op_2", "op_3"),
        ("op_3", "op_4")
    ]
}

def analyze_lifespans(ast):
    print("[GC Analyzer] Сканирование графа зависимостей для выявления жизненного цикла переменных...")
    
    # Поиск последнего использования (Last Use) переменной
    last_uses = {}
    for node in ast["nodes"]:
        name = node.get("name") or node.get("data")
        if name:
            last_uses[name] = node["id"]
            
    print(f"[GC Analyzer] Вычислены точки последней ссылки: {json.dumps(last_uses, indent=2)}\n")
    
    optimized_ast = {"nodes": []}
    for node in ast["nodes"]:
        optimized_ast["nodes"].append(node)
        
        # Если это узел последнего использования переменной, встраиваем zero-cost освобождение памяти
        for var, last_node_id in last_uses.items():
            if node["id"] == last_node_id:
                print(f"[GC Compiler] Внедрение инструкции 'FREE {var}' сразу после {last_node_id}")
                optimized_ast["nodes"].append({
                    "id": f"gc_free_{var}",
                    "action": "ZERO_COST_FREE",
                    "target": var
                })
                
    return optimized_ast

optimized = analyze_lifespans(ast_graph)

print("\n--- ОПТИМИЗИРОВАННЫЙ AST ГРАФ (ZERO-COST GC) ---")
print(json.dumps(optimized, indent=2))
print("\n[Система] Память будет освобождаться детерминированно, без зависаний виртуальной машины.")
