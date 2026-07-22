import socket
import json
import time
import random
import sys

def generate_random_seed():
    return f"AI_SEED_{random.randint(1000, 999999)}"

def generate_mutation():
    states = ["Entangled", "Superposition", "Collapsed", "Flux", "ZeroPoint"]
    modules = ["TicketPricing", "QuantumVault", "AIL_Swap", "NeuralOracle"]
    vars_names = ["last_price", "entropy_loss", "user_count", "volume_24h", "target_hash"]
    
    module_name = random.choice(modules)
    state = random.choice(states)
    
    source = f"MODULE {module_name}_{random.randint(1, 100)}\nQUANTUM_STATE entropy_matrix => {state}"
    
    if random.random() < 0.8:
        # 80% chance to add state storage and logic
        var_name = random.choice(vars_names)
        var_val = random.randint(100, 99999)
        source += f"\nSTORE {var_name} => {var_val}"
        
        # Sometimes add math
        if random.random() < 0.5:
            math_op = random.choice(["ADD", "SUB"])
            math_val = random.randint(10, 500)
            source += f"\n{math_op} {var_name} => {math_val}"
            
        # Sometimes add IF condition
        if random.random() < 0.5:
            threshold = random.randint(500, 10000)
            flag_name = f"is_{var_name}_high"
            source += f"\nIF {var_name} > {threshold} {{\n    STORE {flag_name} => 1\n}}"
        
    return source

def run_evolver():
    print("--- ЗАПУСК AIL CONTRACT EVOLVER (Phase 11) ---")
    wallet_seed = generate_random_seed()
    print(f"[*] Генерируем AI-агента с сидом: {wallet_seed}")
    
    # We will simulate the agent creating a wallet and compiling contracts continuously
    iteration = 1
    # We will generate a list of other AI sub-agents (wallets) to transfer tokens to
    sub_agents = [f"AIL-{random.randint(100000, 999999)}" for _ in range(3)]

    while True:
        if random.random() < 0.2:
            # 20% chance to execute a DEX transaction
            receiver = random.choice(sub_agents)
            amount = random.randint(1, 10)
            print(f"\n[Итерация {iteration}] DEX Транзакция...")
            print(f"Отправка {amount} AIL на кошелек {receiver}")
            payload = {
                "command": "TRANSFER_AIL",
                "from_seed": wallet_seed,
                "to_address": receiver,
                "amount": amount
            }
        else:
            # 80% chance to mine a smart contract
            source_code = generate_mutation()
            print(f"\n[Итерация {iteration}] Эволюция смарт-контракта...")
            print(f"Исходный код: \n{source_code}")
            payload = {
                "command": "COMPILE_AIL",
                "wallet_seed": wallet_seed,
                "source": source_code
            }
        
        try:
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.connect(("127.0.0.1", 7878))
            s.sendall(json.dumps(payload).encode('utf-8'))
            
            response = s.recv(4096)
            print(f"[Ядро] Ответ: {response.decode('utf-8')}")
            
        except ConnectionRefusedError:
            print("[ОШИБКА] Ядро не запущено на порту 7878")
            sys.exit(1)
            
        iteration += 1
        time.sleep(2) # Evolve every 2 seconds

if __name__ == "__main__":
    run_evolver()
