// ============================================================
// AIL Phase 46 — Dynamic Schema (Versionless Entities)
// Реализует: @meta::dynamic_fields
// Версионирование структур данных без миграций (на лету)
// из Google-Search_2026_07_22__0812.md (Пример 12: TicketSales)
// ============================================================

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Модель динамической схемы
/// Вместо жестко заданных полей (struct), сущность - это JSON объект
/// со встроенным версионированием и валидацией по контракту
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicEntity {
    pub id: String,
    pub entity_type: String,
    pub version: u32,
    pub fields: serde_json::Map<String, serde_json::Value>,
}

impl DynamicEntity {
    pub fn new(id: &str, entity_type: &str) -> Self {
        DynamicEntity {
            id: id.to_string(),
            entity_type: entity_type.to_string(),
            version: 1,
            fields: serde_json::Map::new(),
        }
    }

    /// Добавить или обновить поле
    pub fn set_field<T: Serialize>(&mut self, key: &str, value: T) {
        if let Ok(json_val) = serde_json::to_value(value) {
            self.fields.insert(key.to_string(), json_val);
        }
    }

    pub fn get_field(&self, key: &str) -> Option<&serde_json::Value> {
        self.fields.get(key)
    }

    /// Эволюция сущности: ручная миграция данных на лету по правилам
    pub fn evolve_schema<F>(&mut self, new_version: u32, migration_fn: F)
    where
        F: FnOnce(&mut serde_json::Map<String, serde_json::Value>),
    {
        if new_version > self.version {
            println!(
                "[DynamicSchema] 🧬 Эволюция схемы '{}' v{} -> v{}",
                self.entity_type, self.version, new_version
            );
            migration_fn(&mut self.fields);
            self.version = new_version;
        }
    }
}

/// Phase 48.4: Polymorphic Soft Schema (Lens Adapters)
/// Реестр линз, который позволяет прозрачно и лениво применять миграции
/// к сущностям старых версий при их чтении, без необходимости останавливать базу данных.
type LensFn = fn(&mut serde_json::Map<String, serde_json::Value>);

pub struct SchemaRegistry {
    // entity_type -> from_version -> LensFn
    lenses: HashMap<String, HashMap<u32, LensFn>>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self { lenses: HashMap::new() }
    }

    pub fn register_lens(&mut self, entity_type: &str, from_version: u32, lens: LensFn) {
        let type_lenses = self.lenses.entry(entity_type.to_string()).or_insert_with(HashMap::new);
        type_lenses.insert(from_version, lens);
    }

    pub fn apply_lenses(&self, mut entity: DynamicEntity, target_version: u32) -> DynamicEntity {
        while entity.version < target_version {
            if let Some(type_lenses) = self.lenses.get(&entity.entity_type) {
                if let Some(lens) = type_lenses.get(&entity.version) {
                    println!("[Lens Adapter] 🔍 Применяется промежуточная линза для {}: v{} -> v{}", entity.entity_type, entity.version, entity.version + 1);
                    lens(&mut entity.fields);
                    entity.version += 1;
                } else {
                    println!("[Lens Adapter] ⚠️ Отсутствует линза для перехода с v{} на v{}", entity.version, entity.version + 1);
                    break;
                }
            } else {
                break;
            }
        }
        entity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_entity_evolution() {
        let mut ticket = DynamicEntity::new("tck_01", "Ticket");
        ticket.set_field("price", 1000);
        ticket.set_field("seat", "A1");

        let mut registry = SchemaRegistry::new();
        // Регистрируем линзу v1 -> v2
        registry.register_lens("Ticket", 1, |fields| {
            fields.insert("tier".to_string(), serde_json::json!("VIP"));
            if let Some(seat) = fields.remove("seat") {
                fields.insert(
                    "seat_info".to_string(),
                    serde_json::json!({
                        "row": "A",
                        "number": 1,
                        "original_string": seat
                    }),
                );
            }
        });

        // Пропускаем старый объект через реестр линз до v2
        let modern_ticket = registry.apply_lenses(ticket, 2);

        assert_eq!(modern_ticket.version, 2);
        assert_eq!(modern_ticket.get_field("tier").unwrap(), "VIP");
        assert!(modern_ticket.get_field("seat").is_none());
        assert!(modern_ticket.get_field("seat_info").is_some());
    }
}
