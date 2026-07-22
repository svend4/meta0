# Multiverse Swarm Automated Launcher
Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host "   🚀 AIL MULTIVERSE SWARM LAUNCHER (4-NODE CLUSTER)" -ForegroundColor Cyan
Write-Host "=========================================================" -ForegroundColor Cyan

$cargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargoPath)) {
    $cargoPath = "cargo"
}

Write-Host "[1/4] Building AIL Runtime..." -ForegroundColor Yellow
Start-Process -FilePath $cargoPath -ArgumentList "build" -WorkingDirectory "c:\xampp\htdocs\01\10\infocolangail\ail_prototype" -NoNewWindow -Wait

Write-Host "[2/4] Launching Bootnode (Port 7879)..." -ForegroundColor Green
$bootnode = Start-Process -FilePath $cargoPath -ArgumentList "run --bin ail_runtime -- 7879" -WorkingDirectory "c:\xampp\htdocs\01\10\infocolangail\ail_prototype" -PassThru

Start-Sleep -Seconds 3

Write-Host "[3/4] Spawning Swarm Nodes (Ports 7878, 7880, 7881)..." -ForegroundColor Green
$node1 = Start-Process -FilePath $cargoPath -ArgumentList "run --bin ail_runtime -- 7878" -WorkingDirectory "c:\xampp\htdocs\01\10\infocolangail\ail_prototype" -PassThru
$node2 = Start-Process -FilePath $cargoPath -ArgumentList "run --bin ail_runtime -- 7880" -WorkingDirectory "c:\xampp\htdocs\01\10\infocolangail\ail_prototype" -PassThru
$node3 = Start-Process -FilePath $cargoPath -ArgumentList "run --bin ail_runtime -- 7881" -WorkingDirectory "c:\xampp\htdocs\01\10\infocolangail\ail_prototype" -PassThru

Write-Host "[4/4] Opening Exocortex Dashboard..." -ForegroundColor Magenta
Start-Process "c:\xampp\htdocs\01\10\infocolangail\ail_prototype\dashboard.html"

Write-Host "`n✅ Multiverse Swarm is running across 4 ports!" -ForegroundColor Green
Write-Host "Press Ctrl+C or kill processes when done." -ForegroundColor Gray
