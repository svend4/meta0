use std::sync::Arc;
use tokio::sync::RwLock;

struct AuthSystem {
    cache: Arc<RwLock<HashMap<String, bool>>>,
}

impl AuthSystem {
    pub async fn check_token(&self, token: &str) -> Result<bool, AuthError> {
        if token.is_empty() {
            return Err(AuthError::InvalidToken);
        }

        // Блокировка для чтения кэша (борьба со сборщиком мусора и потоками)
        let cache_read = self.cache.read().await;
        if let Some(&is_valid) = cache_read.get(token) {
            return Ok(is_valid);
        }
        drop(cache_read); // Явно освобождаем память

        // Тяжелая проверка...
        let is_valid = self.verify_via_crypto(token)?;

        let mut cache_write = self.cache.write().await;
        cache_write.insert(token.to_string(), is_valid);

        Ok(is_valid)
    }
}