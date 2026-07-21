// Реальный пример концепта верификации в Rust (2026)
#[kani::proof]
fn verify_transfer_logic() {
    let mut account_a = 100;
    let mut account_b = 0;
    let amount = kani::any::<u32>(); // ИИ-тестер симулирует ЛЮБОЕ число

    // Математический контракт
    if amount <= account_a {
        account_a -= amount;
        account_b += amount;
        // Проверка инварианта: общая сумма не изменилась
        assert_eq!(account_a + account_b, 100);
    }
}