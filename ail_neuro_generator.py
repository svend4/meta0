import socket
import json
import time
import random

print("--- ЗАПУСК НЕЙРО-СИНТЕЗАТОРА AST-ГРАФОВ (PYTHON) ---")
print("[AI] Обучение на 47 спецификациях завершено. Генерация потока...")

# Целевой сервер Rust-ядра
HOST = '127.0.0.1'
PORT = 7878

def generate_ast():
    # Имитируем ИИ-генерацию полезной нагрузки
    is_hacker = random.random() < 0.3
    
    ast_id = f"AST_GEN_{random.randint(1000, 9999)}"
    amount = random.randint(100, 50000)
    
    # Хакерский узел попытается прислать невалидный ZKP
    zkp_proof = "INVALID_ZKP_HACK_ATTEMPT" if is_hacker else f"VALID_ZKP_{random.randint(100000, 999999)}"
    
    return {
        "ast_id": ast_id,
        "amount_req": amount,
        "zkp_proof": zkp_proof,
        "is_cross_border": random.choice([True, False])
    }

def send_payload(payload):
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.connect((HOST, PORT))
            
            # Отправка JSON
            data = json.dumps(payload)
            s.sendall(data.encode('utf-8'))
            
            # Ожидание ответа от Rust
            response = s.recv(1024)
            print(f"[Ответ от Ядра AIL]: {response.decode('utf-8')}")
            
    except ConnectionRefusedError:
        print("[Ошибка] Ядро AIL на Rust еще не запущено или не слушает порт 7878.")

# Запускаем генерацию и отправку
for i in range(1, 6):
    payload = generate_ast()
    print(f"\n[AI Synthesizer] Создан новый AST Граф: {payload['ast_id']}. Отправка в Ядро...")
    send_payload(payload)
    time.sleep(1)

print("\n[AI Synthesizer] Сеанс генерации завершен.")
