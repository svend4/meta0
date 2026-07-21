import socket
import json
import time
import random

print("--- ЗАПУСК ГЕНЕТИЧЕСКОГО ИНКУБАТОРА AST (PYTHON) ---")
print("[Эволюция] Инициализация пула мутаций...")

HOST = '127.0.0.1'
PORT = 7878

def send_payload(payload):
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.connect((HOST, PORT))
            s.sendall(json.dumps(payload).encode('utf-8'))
            response = s.recv(1024)
            resp_data = json.loads(response.decode('utf-8'))
            return resp_data
    except Exception as e:
        print(f"[Ошибка подключения] {e}")
        return None

def evolve_ast():
    # Создаем базовый ошибочный геном (AST с неверным ZKP)
    base_ast_id = f"AST_MUTANT_{random.randint(1000, 9999)}"
    genome = {
        "ast_id": base_ast_id,
        "amount_req": 50000,
        "zkp_proof": "INVALID_GENOME_STRAND",
        "is_cross_border": True,
        "mutation_gen": 0
    }
    
    generation = 0
    while generation < 5:
        generation += 1
        genome["mutation_gen"] = generation
        print(f"\n[Generation {generation}] Отправка мутанта {base_ast_id} в Ядро...")
        
        resp = send_payload(genome)
        if not resp:
            break
            
        print(f"[Ядро] Ответ: {resp['status']}")
        
        if resp['code'] == 200:
            print(f"[Эволюция УСПЕШНА] Геном {base_ast_id} адаптировался на поколении {generation}!")
            return True
        else:
            print(f"[Эволюция ПРОВАЛ] Ядро уничтожило геном. Запуск случайной мутации...")
            # Мутация генома: шанс 30% подобрать правильный крипто-ключ
            if random.random() < 0.3:
                genome["zkp_proof"] = f"VALID_ZKP_EVOLVED_{random.randint(100,999)}"
            time.sleep(1)
            
    print(f"\n[Смерть] Мутант {base_ast_id} не смог адаптироваться за 5 поколений и был удален.")
    return False

# Запускаем 3 независимых генетических эксперимента
for i in range(1, 4):
    print(f"\n--- ЗАПУСК ЭКСПЕРИМЕНТА #{i} ---")
    evolve_ast()
    time.sleep(2)

print("\n[Инкубатор] Сеанс генетической эволюции завершен.")
