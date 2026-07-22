$ports = @(7878, 7879, 7880)

Write-Host "--- ЗАПУСК AIL TESTNET ---" -ForegroundColor Cyan
Write-Host "Компиляция Ядра..." -ForegroundColor Yellow

cargo build

if ($LASTEXITCODE -ne 0) {
    Write-Host "Ошибка компиляции!" -ForegroundColor Red
    exit 1
}

Write-Host "Компиляция завершена. Запуск 3 узлов (Nodes)..." -ForegroundColor Green

foreach ($port in $ports) {
    Write-Host "Запуск Ноды на порту $port..." -ForegroundColor Magenta
    $envVars = @{ "PORT" = "$port" }
    if ($port -ne 7878) {
        $envVars["SEED_NODE"] = "127.0.0.1:7878"
    }
    Start-Process -FilePath "target\debug\ail_runtime.exe" -Environment $envVars
}

Write-Host "Все 3 Ноды запущены в отдельных окнах!" -ForegroundColor Cyan
Write-Host "Теперь вы можете запустить python ail_contract_evolver.py" -ForegroundColor Yellow
