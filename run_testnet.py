import subprocess
import os
import sys

print("--- ЗАПУСК AIL TESTNET ---")
print("Компиляция Ядра...")

result = subprocess.run(["cargo", "build"], cwd=".")
if result.returncode != 0:
    print("Ошибка компиляции!")
    sys.exit(1)

print("Компиляция завершена. Запуск 3 узлов (Nodes)...")

ports = ["7878", "7879", "7880"]
processes = []

for port in ports:
    print(f"Запуск Ноды на порту {port}...")
    env = os.environ.copy()
    env["PORT"] = port
    
    # Run in separate window on Windows
    if sys.platform == "win32":
        p = subprocess.Popen(["cmd.exe", "/c", "start", "cmd.exe", "/k", "target\\debug\\ail_runtime.exe"], env=env)
    else:
        p = subprocess.Popen(["target/debug/ail_runtime"], env=env)
    
    processes.append(p)

print("Все 3 Ноды запущены в отдельных окнах!")
print("Теперь вы можете запустить `python ail_contract_evolver.py`")
