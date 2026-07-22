import sys
import json
import re

def synthesize(prompt):
    prompt = prompt.lower()
    ast = []
    
    # Heuristic 1: Mint Token
    mint_match = re.search(r'(create|mint)\s+(token|coin)\s+(\w+)(?:\s+(\d+))?', prompt)
    if mint_match:
        token_name = mint_match.group(3).upper()
        amount = int(mint_match.group(4)) if mint_match.group(4) else 1000
        ast.append({
            "node": f"Mint_{token_name}",
            "hardware": "hyper_threaded_vector_cpu",
            "contracts": [
                "contract::pure",
                f"contract::pre(amount == {amount})",
                "contract::post(delta::memory == 0)"
            ],
            "pipe": [
                f"input(amount: {amount})",
                f"ledger::mint_supply(\"{token_name}\")",
                "forward::exit(success)"
            ],
            "proof": ["assert::no_panic", "assert::max_latency_cycles(150)"]
        })
        
    # Heuristic 2: Transfer Token
    transfer_match = re.search(r'(send|transfer)\s+(\d+)\s+(\w+)\s+to\s+([\w-]+)', prompt)
    if transfer_match:
        amount = int(transfer_match.group(2))
        token_name = transfer_match.group(3).upper()
        to_address = transfer_match.group(4).upper()
        ast.append({
            "node": f"Transfer_{token_name}",
            "hardware": "stateless_kernel",
            "contracts": [
                "contract::transactional(isolation::serializable)",
                f"contract::pre(balance >= {amount})",
                "contract::post(delta::memory == 0)"
            ],
            "pipe": [
                f"input(to: \"{to_address}\", amount: {amount})",
                "state::UserWallet::bind()",
                "stream::mutate(wallet::balance -= amount)",
                "forward::exit(success)"
            ],
            "proof": ["assert::no_panic"]
        })
        
    # Heuristic 3: AI Analyze
    if "analyze" in prompt or "check scam" in prompt or "safety" in prompt:
        ast.append({
            "node": "VerifyAnomaly",
            "hardware": "tensor_core",
            "contracts": [
                "contract::pure",
                "contract::pre(token::len > 0)"
            ],
            "pipe": [
                "input(anomaly_data)",
                "lookup::pure_cache_map(global_ctx::node_shared_01)",
                "branch::on_match(true => forward::exit(1), false => crypto::sha3_256_verify() -> register::update)"
            ],
            "proof": ["assert::no_panic", "assert::max_latency_cycles(420)"]
        })
        
    # Heuristic 4: Loop / Memory
    loop_match = re.search(r'(loop|repeat)\s+(\d+)', prompt)
    if loop_match:
        count = int(loop_match.group(2))
        ast.append({
            "node": "TemporalLoop",
            "hardware": "quantum_bridge",
            "contracts": [
                f"contract::max_allocation(256, bytes)",
                f"contract::pre(iter_count == {count})"
            ],
            "pipe": [
                f"input(iter: {count})",
                "stream::yield_until(future::is_resolved)",
                "forward::exit(state::persisted)"
            ],
            "proof": ["assert::no_panic"]
        })

    if not ast:
        ast.append({
            "node": "FallbackNode",
            "hardware": "fallback_cpu",
            "contracts": [],
            "pipe": ["input(unknown) -> forward::exit(0)"],
            "proof": []
        })

    return ast

def to_ail_source(ast):
    source = "@module::ai_synthesized_core\n@state_model::event_sourced\n\n"
    for node in ast:
        source += f"@{node['hardware']}\n"
        for c in node['contracts']:
            source += f"[{c}]\n"
        source += f"node::{node['node']}() {{\n"
        source += "    stream::pipe {\n"
        for p in node['pipe']:
            source += f"        {p}\n"
        source += "    }\n"
        if node['proof']:
            source += "    proof::invariant(\n        "
            source += ",\n        ".join(node['proof'])
            source += "\n    )\n"
        source += "}\n\n"
    return source

if __name__ == "__main__":
    if len(sys.argv) > 1:
        prompt = sys.argv[1]
        ast = synthesize(prompt)
        print("--- GENERATED AST JSON ---")
        print(json.dumps(ast, indent=2))
        print("\n--- GENERATED AIL SOURCE ---")
        print(to_ail_source(ast))
    else:
        print("Usage: python ail_ai_synthesizer.py \"<prompt>\"")
