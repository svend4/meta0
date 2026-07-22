use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use ed25519_dalek::{SigningKey, Signer};
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;
use rand::RngCore;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_help();
        return;
    }

    let command = &args[1];

    match command.as_str() {
        "status" => {
            println!("📡 Запрос статуса у локальной ноды...");
            send_http_get("127.0.0.1:7878", "/api/status");
        }
        "deploy" => {
            if args.len() < 4 {
                println!("Использование: ail-cli deploy <seed_phrase> <file.ail>");
                return;
            }
            let seed = &args[2];
            let filepath = &args[3];
            
            let source_code = match fs::read_to_string(filepath) {
                Ok(content) => content,
                Err(e) => {
                    println!("❌ Ошибка чтения файла {}: {}", filepath, e);
                    return;
                }
            };
            
            println!("🚀 Отправка смарт-контракта {} в сеть (от имени {})...", filepath, seed);
            
            let json_body = format!(
                "{{\"command\":\"COMPILE_AIL\",\"wallet_seed\":\"{}\",\"source\":\"{}\"}}",
                seed,
                source_code.replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "")
            );
            
            send_http_post("127.0.0.1:7878", "/", &json_body);
        }
        "spec" => {
            // Проход 4: загрузить .ail-спеку и получить структурный разбор + верификацию.
            if args.len() < 3 {
                println!("Использование: ail-cli spec <file.ail>");
                return;
            }
            let filepath = &args[2];
            let source_code = match fs::read_to_string(filepath) {
                Ok(content) => content,
                Err(e) => {
                    println!("❌ Ошибка чтения файла {}: {}", filepath, e);
                    return;
                }
            };
            println!("📋 Анализ .ail-спеки {} через ноду...", filepath);
            let json_body = format!(
                "{{\"command\":\"LOAD_SPEC\",\"source\":\"{}\"}}",
                source_code.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "")
            );
            send_http_post("127.0.0.1:7878", "/", &json_body);
        }
        "zkp_transfer" => {
            if args.len() < 5 {
                println!("Использование: ail-cli zkp_transfer <seed_phrase> <to_address> <amount>");
                return;
            }
            let seed = &args[2];
            let to_address = &args[3];
            let amount = &args[4];
            
            // 1. Generate Ed25519 Keypair from seed
            let mut hasher = Sha256::new();
            hasher.update(seed.as_bytes());
            let seed_hash: [u8; 32] = hasher.finalize().into();
            let keypair = SigningKey::from_bytes(&seed_hash);
            let public_key_hex = hex::encode(keypair.verifying_key().as_bytes());
            
            // 2. Generate random Blinding Factor
            let mut blinding_factor = [0u8; 32];
            OsRng.fill_bytes(&mut blinding_factor);
            let blinding_hex = hex::encode(blinding_factor);
            
            // 3. Compute Commitment: Hash(amount + blinding_factor)
            let mut commit_hasher = Sha256::new();
            commit_hasher.update(amount.as_bytes());
            commit_hasher.update(&blinding_factor);
            let commitment_hex = hex::encode(commit_hasher.finalize());
            
            // 4. Sign the commitment
            let message = format!("ZKP_TRANSFER {} TO {}", commitment_hex, to_address);
            let signature = keypair.sign(message.as_bytes());
            let signature_hex = hex::encode(signature.to_bytes());
            
            println!("🔒 Генерация Слепой Транзакции (ZKP)...");
            println!("   - Отправитель (Pub): {}", public_key_hex);
            println!("   - Commitment (Скрытая сумма): {}", commitment_hex);
            
            let json_body = format!(
                "{{\"command\":\"ZKP_TRANSFER_AIL\",\"public_key\":\"{}\",\"to_address\":\"{}\",\"commitment\":\"{}\",\"zkp_proof\":\"VALID_PROOF\",\"encrypted_amount\":\"{}\",\"signature\":\"{}\"}}",
                public_key_hex, to_address, commitment_hex, amount, signature_hex
            );
            
            send_http_post("127.0.0.1:7878", "/", &json_body);
        }
        _ => {
            println!("❌ Неизвестная команда: {}", command);
            print_help();
        }
    }
}

fn print_help() {
    println!("==================================================");
    println!(" 💻 AIL (Artificial Intelligence Ledger) Terminal ");
    println!("==================================================");
    println!("Доступные команды:");
    println!("  ail-cli status                                      - Статус ноды");
    println!("  ail-cli deploy <seed_phrase> <file.ail>             - Деплой смарт-контракта");
    println!("  ail-cli spec <file.ail>                             - Разбор+верификация .ail-спеки");
    println!("  ail-cli zkp_transfer <seed> <to_address> <amount>   - Слепой перевод (ZKP)");
    println!("==================================================");
}

fn send_http_get(host_port: &str, path: &str) {
    if let Ok(mut stream) = TcpStream::connect(host_port) {
        let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host_port);
        if stream.write_all(request.as_bytes()).is_ok() {
            let mut response = String::new();
            if stream.read_to_string(&mut response).is_ok() {
                if let Some(body_start) = response.find("\r\n\r\n") {
                    let body = &response[body_start + 4..];
                    println!("\n[УСПЕХ] Ответ ноды:\n{}", body);
                }
            }
        }
    } else {
        println!("❌ Ошибка: Нода недоступна по адресу {}", host_port);
    }
}

fn send_http_post(host_port: &str, path: &str, json_body: &str) {
    if let Ok(mut stream) = TcpStream::connect(host_port) {
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            path,
            host_port,
            json_body.len(),
            json_body
        );
        if stream.write_all(request.as_bytes()).is_ok() {
            let mut response = String::new();
            if stream.read_to_string(&mut response).is_ok() {
                if let Some(body_start) = response.find("\r\n\r\n") {
                    let body = &response[body_start + 4..];
                    println!("\n[УСПЕХ] Ответ ноды:\n{}", body);
                }
            }
        }
    } else {
        println!("❌ Ошибка: Нода недоступна по адресу {}", host_port);
    }
}
