import sys
import json
try:
    from z3 import *
except ImportError:
    print(json.dumps({"status": "Failed", "reason": "z3-solver not installed", "errors": []}))
    sys.exit(1)

def main():
    if len(sys.argv) > 1:
        # read json from string arg
        data_str = sys.argv[1]
    else:
        # read from stdin
        data_str = sys.stdin.read()
        
    try:
        req = json.loads(data_str)
    except Exception as e:
        print(json.dumps({"status": "Failed", "reason": f"Invalid JSON: {str(e)}", "errors": []}))
        sys.exit(1)
        
    ast = req.get("ast", [])
    max_alloc = req.get("max_allocation", 999999999999)
    
    solver = Solver()
    
    # We will track symbolic variables
    sym_vars = {}
    
    def get_var(name):
        if name not in sym_vars:
            sym_vars[name] = Int(name)
        return sym_vars[name]

    estimated_memory = 0
    errors = []
    
    # Helper to parse serde externally tagged enum
    def parse_node(node):
        if isinstance(node, dict):
            ntype = list(node.keys())[0]
            val = node[ntype]
            return ntype, val
        elif isinstance(node, str):
            return node, None
        return None, None

    # 1. Parse definitions and initial constraints
    for node in ast:
        ntype, val = parse_node(node)
        if not ntype: continue
            
        if ntype == "ContractPre":
            var = get_var(val["var_name"])
            op = val["operator"]
            limit = int(val["limit"])
            if op == ">": solver.add(var > limit)
            elif op == "<": solver.add(var < limit)
            elif op == ">=": solver.add(var >= limit)
            elif op == "<=": solver.add(var <= limit)
            elif op == "==": solver.add(var == limit)
            
        elif ntype == "StoreState":
            var = get_var(val[0])
            value = int(val[1])
            solver.add(var == value)
            estimated_memory += 32
            
    if estimated_memory > max_alloc:
        errors.append({
            "error_node": "GlobalAllocation",
            "reason": f"Memory overflow. Estimated: {estimated_memory}, Limit: {max_alloc}",
            "risk_type": "MemoryOverflow"
        })
        
    # 2. Check mutations and look for invariant violations (like i128::MAX)
    I128_MAX = (2**127) - 1
    I128_MIN = -(2**127)
    
    current_versions = {k: v for k, v in sym_vars.items()}
    
    step = 0
    for node in ast:
        ntype, val = parse_node(node)
        if not ntype: continue
            
        if ntype == "MathAdd":
            var_name = val[0]
            delta = int(val[1])
            old_var = current_versions.get(var_name, get_var(var_name))
            new_var = Int(f"{var_name}_step{step}")
            solver.add(new_var == old_var + delta)
            current_versions[var_name] = new_var
            
            # Check overflow
            push_solver = Solver()
            push_solver.add(solver.assertions())
            push_solver.add(new_var > I128_MAX)
            if push_solver.check() == sat:
                m = push_solver.model()
                problem_val = m.evaluate(old_var).as_long() if m[old_var] is not None else 0
                errors.append({
                    "error_node": f"MathAdd({var_name})",
                    "reason": f"Integer overflow risk: {var_name} + {delta} > i128::MAX",
                    "counter_example": {
                        "variable": var_name,
                        "problematic_value": problem_val,
                        "description": "Сложение превышает i128::MAX"
                    },
                    "suggested_fix_ast_patch": f"insert [contract::pre({var_name} + {delta} <= i128::MAX)]"
                })
            
        elif ntype == "MathSub":
            var_name = val[0]
            delta = int(val[1])
            old_var = current_versions.get(var_name, get_var(var_name))
            new_var = Int(f"{var_name}_step{step}")
            solver.add(new_var == old_var - delta)
            current_versions[var_name] = new_var
            
            # Check underflow
            push_solver = Solver()
            push_solver.add(solver.assertions())
            push_solver.add(new_var < I128_MIN)
            if push_solver.check() == sat:
                m = push_solver.model()
                problem_val = m.evaluate(old_var).as_long() if m[old_var] is not None else 0
                errors.append({
                    "error_node": f"MathSub({var_name})",
                    "reason": f"Integer underflow risk: {var_name} - {delta} < i128::MIN",
                    "counter_example": {
                        "variable": var_name,
                        "problematic_value": problem_val,
                        "description": "Вычитание ниже i128::MIN"
                    },
                    "suggested_fix_ast_patch": f"insert [contract::pre({var_name} >= {delta})]"
                })
        step += 1
        
    status = "Passed" if len(errors) == 0 else "Failed"
    
    print(json.dumps({
        "status": status,
        "module": "z3_verified_module",
        "theorems_proven": len(solver.assertions()),
        "theorems_failed": len(errors),
        "errors": errors,
        "estimated_memory_bytes": estimated_memory
    }))

if __name__ == "__main__":
    main()
