// ============================================================
// AIL Проход 2 — клиент бинарного транспорта (Binary Delta Streaming).
//
// Честная версия client.rs из лога Google-Search_2026_07_22: тот не
// компилировался (stream::Write::write_all — несуществующий путь) и хардкодил
// параметры. Здесь: типы берутся из lib-крейта (одно определение на клиент
// и сервер), кадры length-prefixed, параметры — из аргументов CLI.
//
// Использование:
//   cargo run --bin ail-binary-client -- [wallet_id] [amount] [port]
//   по умолчанию: wallet-user-999 1500 7978
// ============================================================

use ail_runtime::shard_engine::{
    read_frame_pub, write_frame, BinaryAstPayload, BinaryTxRequest, TxResult,
};
use std::net::TcpStream;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let wallet_id = args.get(1).cloned().unwrap_or_else(|| "wallet-user-999".to_string());
    let amount: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(1500);
    let port: u16 = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(7978);

    let req = BinaryTxRequest {
        wallet_id: wallet_id.clone(),
        amount,
        payload: BinaryAstPayload::debit(),
    };

    // Честное сравнение размеров транспорта (в логе «~40 байт» были декларацией).
    let bin_bytes = bincode::serialize(&req).expect("bincode serialize");
    let json_bytes = serde_json::to_vec(&req).expect("json serialize");
    println!("[BinaryClient] 📦 Размер графа: bincode = {} байт, JSON = {} байт ({:.1}x)",
        bin_bytes.len(),
        json_bytes.len(),
        json_bytes.len() as f64 / bin_bytes.len() as f64
    );

    let addr = format!("127.0.0.1:{}", port);
    let mut stream = match TcpStream::connect(&addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[BinaryClient] ❌ Нет соединения с {} — нода запущена? ({})", addr, e);
            std::process::exit(1);
        }
    };

    write_frame(&mut stream, &bin_bytes).expect("send frame");
    let resp = read_frame_pub(&mut stream).expect("read response frame");
    let result: TxResult = bincode::deserialize(&resp).expect("decode TxResult");

    println!(
        "[BinaryClient] 📨 Ответ: code={} | shard={} | new_balance={} | {}",
        result.code, result.shard_id, result.new_balance, result.message
    );
}
