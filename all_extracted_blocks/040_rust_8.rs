use std::net::TcpStream;
use std::io::{Write, Read};
use serde::{Deserialize, Serialize};

// Структуры данных должны ИДЕНТИЧНО повторять серверные типы для корректного маппинга памяти
#[derive(Serialize, Deserialize, Debug, Clone)]
enum AilBinaryOp {
    #[serde(rename = "SYS_STATE_BIND")]
    SysStateBind { target_store: String },

    #[serde(rename = "MAP_GET_PROPERTY")]
    MapGetProperty { property: String },

    #[serde(rename = "MATH_SUB_SAFE_PROOF")]
    MathSubSafeProof,

    #[serde(rename = "FORWARD_EXIT")]
    ForwardExit { exit_code: u16 },

    #[serde(rename = "CONDITIONAL_BRANCH")]
    ConditionalBranch {
        on_overflow_code: u16,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BinaryAstPayload {
    node_id: u32,
    operations: Vec<AilBinaryOp>,
}

fn main() {
    println!("--- ЗАПУСК ИИ-КЛИЕНТА ПЕРЕДАТЧИКА AIL (ИЮЛЬ 2026) --- \n");

    // 1. ИИ генерирует логический граф на лету (Бизнес-логика: проверить и списать)
    let ai_generated_ast = BinaryAstPayload {
        node_id: 88812,
        operations: vec![
            AilBinaryOp::SysStateBind { target_store: "GlobalLedger".to_string() },
            AilBinaryOp::MapGetProperty { property: "balance".to_string() },
            AilBinaryOp::MathSubSafeProof,
            AilBinaryOp::ConditionalBranch { on_overflow_code: 402 },
        ],
    };

    // 2. Сверхкомпактная бинарная сериализация через Bincode
    // Вместо сотен байт JSON этот граф превращается в крошечный массив из ~40 байт
    let serialized_bytecode: Vec<u8> = bincode::serialize(&ai_generated_ast).unwrap();
    println!("Размер бинарного пакета AST-графа: {} байт", serialized_bytecode.len());

    // 3. Установка прямого TCP-соединения с ВМ
    println!("Подключение к серверу AIL Runtime (127.0.0.1:9999)...");
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:9999") {
        println!("Соединение установлено. Отправка бинарного кода напрямую в процессор...");

        // Выстреливаем байты в сетевую карту
        stream::Write::write_all(&mut stream, &serialized_bytecode).unwrap();
        stream::Write::flush(&mut stream).unwrap();

        // Ожидаем ответ от ВМ сервера
        let mut response_buffer = [0; 512];
        match stream.read(&mut response_buffer) {
            Ok(bytes_read) => {
                let server_response = String::from_utf8_lossy(&response_buffer[..bytes_read]);
                println!("\n[ОТВЕТ СЕРВЕРА]: {}", server_response);
            },
            Err(e) => println!("Ошибка чтения ответа: {}", e),
        }
    } else {
        println!("Ошибка: Сервер AIL Runtime не запущен на порту 9999!");
    }
}