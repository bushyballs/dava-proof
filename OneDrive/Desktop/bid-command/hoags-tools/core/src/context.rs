use serde_json::Value;
use std::path::Path;

pub fn load_context_file(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

pub fn load_context(source: Value) -> Value {
    source
}

pub fn resolve_key(ctx: &Value, dotted_key: &str) -> Option<String> {
    let parts: Vec<&str> = dotted_key.split('.').collect();
    let mut current = ctx;
    for part in parts {
        match current.get(part) {
            Some(v) => current = v,
            None => return None,
        }
    }
    match current {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => Some(current.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resolve_key_dotted() {
        let ctx = json!({"identity": {"name": "Hoags Inc.", "phone": "(458) 239-3215"}});
        assert_eq!(resolve_key(&ctx, "identity.name"), Some("Hoags Inc.".into()));
        assert_eq!(resolve_key(&ctx, "identity.phone"), Some("(458) 239-3215".into()));
    }

    #[test]
    fn test_resolve_key_missing() {
        let ctx = json!({"identity": {"name": "Hoags Inc."}});
        assert_eq!(resolve_key(&ctx, "identity.fax"), None);
        assert_eq!(resolve_key(&ctx, "nonexistent.key"), None);
    }

    #[test]
    fn test_resolve_key_top_level() {
        let ctx = json!({"date": "04/08/2026"});
        assert_eq!(resolve_key(&ctx, "date"), Some("04/08/2026".into()));
    }

    #[test]
    fn test_resolve_key_number_value() {
        let ctx = json!({"quantity": 5});
        assert_eq!(resolve_key(&ctx, "quantity"), Some("5".into()));
    }

    #[test]
    fn test_resolve_key_bool_value() {
        let ctx = json!({"active": true});
        assert_eq!(resolve_key(&ctx, "active"), Some("true".into()));
    }

    #[test]
    fn test_resolve_key_deeply_nested() {
        let ctx = json!({"a": {"b": {"c": "deep_value"}}});
        assert_eq!(resolve_key(&ctx, "a.b.c"), Some("deep_value".into()));
    }

    #[test]
    fn test_resolve_key_partial_path_returns_none() {
        let ctx = json!({"a": {"b": "val"}});
        // "a.b.c" doesn't exist — b is a string, not an object
        assert_eq!(resolve_key(&ctx, "a.b.c"), None);
    }

    #[test]
    fn test_resolve_key_empty_context() {
        let ctx = json!({});
        assert_eq!(resolve_key(&ctx, "anything"), None);
    }

    #[test]
    fn test_resolve_key_array_value() {
        // Arrays are returned as their JSON representation
        let ctx = json!({"tags": ["a", "b"]});
        let result = resolve_key(&ctx, "tags");
        assert!(result.is_some());
        assert!(result.unwrap().contains("a"));
    }
}
