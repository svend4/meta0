// ============================================================
// AIL Phase 46 — Event Sourcing Engine
// Реализует: @state_model::event_sourced
// Логирование всех мутаций как иммутабельной последовательности событий
// из Google-Search_2026_07_22__0812.md (Пример 12)
// ============================================================

use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Единица изменения состояния (Event)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMutation {
    pub event_id: String,
    pub timestamp_ns: u128,
    pub entity_type: String,
    pub entity_id: String,
    pub mutation_type: String, // "CREATED", "UPDATED", "DELETED"
    pub payload_json: String,  // Дельта изменений
    pub user_address: String,
}

/// Движок Event Sourcing (Event Store)
pub struct EventStore {
    log_file: String,
}

impl EventStore {
    pub fn new(log_file: &str) -> Self {
        EventStore {
            log_file: log_file.to_string(),
        }
    }

    /// Записать новое событие (append-only)
    pub fn append_event(
        &self,
        entity_type: &str,
        entity_id: &str,
        mutation_type: &str,
        payload: &str,
        user_address: &str,
    ) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        
        let random_part = ts % 100000;
        let event_id = format!("evt_{}_{}", ts, random_part);

        let mutation = StateMutation {
            event_id,
            timestamp_ns: ts,
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            mutation_type: mutation_type.to_string(),
            payload_json: payload.to_string(),
            user_address: user_address.to_string(),
        };

        if let Ok(json_line) = serde_json::to_string(&mutation) {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_file)
                .unwrap();
            let _ = writeln!(file, "{}", json_line);
            
            println!(
                "[EventSourcing] 📝 Записано событие: {} | Entity: {}/{} | Type: {}",
                mutation.event_id, entity_type, entity_id, mutation_type
            );
        }
    }

    /// Восстановить текущее состояние сущности (Projection)
    /// Проигрывает все события (Event Replay)
    pub fn replay_entity_state(&self, _entity_type: &str, _entity_id: &str) -> Option<serde_json::Value> {
        use std::io::{BufRead, BufReader};
        
        if let Ok(file) = std::fs::File::open(&self.log_file) {
            let reader = BufReader::new(file);
            let mut current_state: Option<serde_json::Value> = None;

            for line in reader.lines().flatten() {
                if let Ok(mut event) = serde_json::from_str::<StateMutation>(&line) {
                    if event.entity_type == _entity_type && event.entity_id == _entity_id {
                        match event.mutation_type.as_str() {
                            "CREATED" | "UPDATED" => {
                                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.payload_json) {
                                    if let Some(state) = &mut current_state {
                                        // Merge updates
                                        if let (Some(s_obj), Some(p_obj)) = (state.as_object_mut(), payload.as_object()) {
                                            for (k, v) in p_obj {
                                                s_obj.insert(k.clone(), v.clone());
                                            }
                                        }
                                    } else {
                                        current_state = Some(payload);
                                    }
                                }
                            }
                            "DELETED" => {
                                current_state = None;
                            }
                            _ => {}
                        }
                    }
                }
            }
            return current_state;
        }
        None
    }
}
