# AIL Runtime — прототип экосистемы Artificial Intelligence Language (`meta0`)

## Назначение проекта

Репозиторий `meta0` содержит сквозной прототип ИИ-ориентированной среды исполнения, компилятора, шардированного акторного движка и P2P-реестра **AIL (Artificial Intelligence Language / Ledger)** (`ail_runtime` v0.3.0). Система сочетает сетевой узел на Rust с формальной верификацией `.ail`-спецификаций, бинарным транспортом `bincode`, криптографическими переводами (Ed25519 и прототипом ZKP), Python-скриптами эволюции/синтеза контрактов и локальными HTML-дашбордами.

---

## Архитектура и основные компоненты

### 1. Rust-ядро и бинарные цели (`Cargo.toml`, `src/`)
- **Основной узел (`src/main.rs`):**
  - Поднимает TCP/HTTP-сервер на порту `PORT` (по умолчанию `7878`) с поддержкой P2P Gossip (`SEED_NODE`, `GOSSIP_HELLO`, `PEERS_LIST`, `SYNC_BLOCK`, `SYNC_MEMPOOL`, `GOSSIP_OMEGA`), фоновым майнером блоков (интервал 5 с) и циклом `Autopoiesis` (интервал 45 с).
  - HTTP API: `GET /api/status`, `GET /api/ast_status`, `GET /api/chrono_state?block_depth=...`, `GET /api/holo_reconstruct?hash=...`, `POST /api/holo_store`, `POST /api/teleport`, `OPTIONS`.
  - Команды ноды: `COMPILE_AIL`, `SYNTHESIZE_AIL` (вызов Python-синтезатора через `CircuitBreaker`), `HOT_SWAP_NODE`, `LOAD_SPEC`, `TRANSFER_AIL`, `ZKP_TRANSFER_AIL`, `ORACLE_SHUTDOWN`, а также обработка `NeuroTransaction`.
- **Библиотека и шардированный движок (`src/lib.rs`, `src/shard_engine.rs`):**
  - Экспортирует `shard_engine` для общих структур клиент-серверного взаимодействия (`ClusterRouter` на 4 акторах через `crossbeam-channel`, бинарные кадры length-prefixed на основе `bincode` на порту `PORT + 100`, по умолчанию `7978`).
- **Компилятор и верификация (`src/compiler/`):**
  - `lexer.rs`, `parser.rs`, `verifier.rs` (формальная проверка контрактов), `sandbox.rs` (`SandboxVerifier`), `inliner.rs` (`ZeroNetworkInliner`), `codegen.rs`, `vm.rs` (`AilVirtualMachine`).
- **Реестр, безопасность и отказоустойчивость (`src/`):**
  - `ledger.rs` (`AilLedger`), `wallet.rs` (`WalletManager`), `smart_contract.rs`, `state_tree.rs` (`AilStateTree`), `zkp_crypto.rs` (`ZkpVerifier`).
  - `circuit_breaker.rs` (`CircuitBreaker` для внешних вызовов Python: порог 3 сбоя, пауза 30 с), `reactive_stream.rs` (`RateLimiter` 100 req/s на IP и `DynamicPricingEngine`), `saga.rs` (`build_ticket_payment_saga`), `dynamic_schema.rs` (`SchemaRegistry` с Lens-миграциями), `event_sourcing.rs` (`EventStore`, журнал `ail_event_log.jsonl`), `holographic_storage.rs` (`HolographicMesh`), `self_healing.rs` (`SelfHealingRegistry` — горячая замена только при прохождении `FormalVerifier` без ослабления контрактов), `spec_loader.rs` (загрузка корпуса `ail_specs/*.ail` при старте узла).
- **Дополнительные и экспериментальные модули (`src/`):**
  - `semantic_router.rs`, `silicon_synth.rs`, `scheduler.rs` (`HardwareScheduler`), `chrono_computing.rs`, `autopoiesis.rs`, `tesseract_memory.rs`, `sentience.rs`, `aether_swarm.rs`, `symbiosis_wasm.rs`, `quantum_memory.rs`, `OMEGA_MANIFESTO.md`.
- **Клиентские утилиты (`src/bin/`):**
  - `ail-cli` (`src/bin/ail_cli.rs`): CLI-клиент к ноде (`127.0.0.1:7878`) с командами `status`, `deploy`, `spec`, `zkp_transfer`.
  - `ail-binary-client` (`src/bin/ail_binary_client.rs`): клиент бинарного `bincode`-транспорта к `127.0.0.1:7978`, выводящий сравнение размера `bincode` и JSON.

### 2. Python-инструменты, спецификации и веб-интерфейсы
- **Скрипты в корне:** `ail_contract_evolver.py` (цикл генерации мутаций контрактов и DEX-транзакций на `127.0.0.1:7878`), `ail_ai_synthesizer.py`, `ail_compiler_trigger.py`, `ail_genetic_evolution.py`, `ail_interpreter_prototype.py`, `ail_neuro_generator.py`, `ail_oracle_mind.py`, `ail_predictive_gc.py`, `benchmark_inliner.py`, `omega_point.py`, `z3_smt_verifier.py`.
- **Спецификации `.ail`:** каталог `ail_specs/` (14 спецификаций: `auth_core_091.ail`, `city_logistics_swarm.ail`, `financial_ledger.ail`, `ticket_booking_core.ail` и др.), а также `test_cli.ail`, `test_verifier.ail`.
- **HTML-интерфейсы:** `dashboard.html`, `exocortex.html`, `exocortex_v2.html`, `wallet.html`.
- **Скрипты тестнета и архивы:** `run_testnet.py`, `run_testnet.ps1`, `run_multiverse_swarm.ps1`, `run_node_1.bat`–`run_node_3.bat`, каталоги `all_extracted_blocks/` и `graveyard/` (архив модулей, исключённых из `main.rs`).

---

## Стек технологий и структура каталогов

- **Rust (edition `2021`):** `serde`, `serde_json`, `bincode`, `crossbeam-channel`, `sha2`, `ed25519-dalek`, `rand`, `hex`.
- **Дополнительно:** Python 3, HTML/JavaScript, PowerShell / Windows Batch.

```text
.
├── Cargo.toml
├── LICENSE / OMEGA_MANIFESTO.md / README.md
├── ail_specs/               # 14 спецификаций .ail
├── all_extracted_blocks/    # извлечённые блоки/артефакты
├── graveyard/               # устаревшие модули, вынесенные из сборки
├── src/
│   ├── lib.rs / main.rs     # библиотека (shard_engine) и ядро узла
│   ├── compiler/            # lexer, parser, verifier, sandbox, inliner, codegen, vm
│   ├── bin/                 # ail_cli.rs, ail_binary_client.rs
│   └── *.rs                 # подсистемы реестра, ZKP, saga, event sourcing, self-healing и др.
├── *.py / *.ps1 / *.bat     # Python-скрипты и запуск локального тестнета
├── *.html                   # dashboard.html, exocortex*.html, wallet.html
└── test_cli.ail / test_verifier.ail
```

---

## Ветки и статус разработки

- `main` *(защищённая ветка по умолчанию)* — единственная ветка в репозитории; открытых Pull Request нет.

---

## Установка и запуск

Требуется установленный инструментарий **Rust / Cargo** (и **Python 3** для вспомогательных скриптов).

### Сборка и тестирование
```bash
cargo build
cargo test
```

### Запуск одиночной ноды и клиентов
```bash
# Запуск узла (HTTP/TCP :7878, бинарный транспорт :7978)
cargo run

# В другом терминале — CLI-клиент к ноде
cargo run --bin ail-cli -- status
cargo run --bin ail-cli -- spec test_verifier.ail
cargo run --bin ail-cli -- deploy my_seed test_cli.ail
cargo run --bin ail-cli -- zkp_transfer my_seed AIL-DEST 100

# Клиент бинарного транспорта (по умолчанию: wallet-user-999 1500 7978)
cargo run --bin ail-binary-client -- wallet-user-999 1500 7978
```

### Запуск локального тестнета (3 узла на портах 7878, 7879, 7880)
```bash
python run_testnet.py
# или в PowerShell на Windows:
# .\run_testnet.ps1

# После запуска узлов — непрерывная отправка мутирующих контрактов:
python ail_contract_evolver.py
```

---

## Лицензия

Проект распространяется под лицензией **GNU Affero General Public License v3.0 (AGPL-3.0)** — подробности см. в файле [LICENSE](LICENSE).
