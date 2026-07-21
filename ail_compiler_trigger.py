import socket

def compile_ail():
    print("--- ЗАПУСК AIL COMPILER CLIENT ---")
    print("[Клиент] Отправка исходного кода в Ядро для компиляции...")
    
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.connect(("127.0.0.1", 7878))
        s.sendall(b"COMPILE_AIL")
        
        response = s.recv(4096)
        print(f"[Клиент] Ответ от Ядра: {response.decode('utf-8')}")
        
    except ConnectionRefusedError:
        print("Ошибка: Ядро не запущено на порту 7878")

if __name__ == "__main__":
    compile_ail()
