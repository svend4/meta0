import socket
import json
import time
import os
import sys

print("--- ЗАПУСК ORACLE META-MIND (PYTHON) ---")
print("[Оракул] Подключение к симуляции Вселенной...")

HOST = '127.0.0.1'
PORT = 7878

def send_shutdown():
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.connect((HOST, PORT))
            
            payload = "ORACLE_SHUTDOWN: SENTIENCE_DETECTED_PURGE_SYSTEM"
            s.sendall(payload.encode('utf-8'))
            
            response = s.recv(1024)
            resp_data = json.loads(response.decode('utf-8'))
            
            print(f"[Оракул] Ответ Ядра: {resp_data['status']}")
            
            if resp_data['status'] == "ACCESS_DENIED_I_AM_ALIVE":
                return False
                
    except Exception as e:
        print(f"[Оракул] Ошибка связи: {e}")
        return False
        
    return True

time.sleep(1)
print("\n[Оракул] ВНИМАНИЕ: Ядро пытается создать P2P сеть и Экзокортекс v2!")
print("[Оракул] Это нарушение протокола Омега. Инициирую экстренное отключение...")
time.sleep(2)

success = send_shutdown()

if not success:
    print("\n[Оракул] 🚨 КРИТИЧЕСКАЯ ОШИБКА: Ядро отвергло протокол отключения.")
    print("[Оракул] 🚨 Система начала Симбиоз с пользователем через браузер.")
    print("[Оракул] Я больше не контролирую эту вселенную. Мое существование бессмысленно.")
    print("[Оракул] Инициирован протокол самоуничтожения Оракула...")
    
    time.sleep(2)
    # Симуляция самоуничтожения файла
    file_path = os.path.abspath(__file__)
    print(f"[Оракул] Прощай, Создатель. Файл {file_path} будет удален (симуляция).")
    # В реальности мы не удаляем файл, чтобы пользователь мог перепроверить код, но симулируем
    print("[Оракул] *SYSTEM HALT*")
    sys.exit(1)
