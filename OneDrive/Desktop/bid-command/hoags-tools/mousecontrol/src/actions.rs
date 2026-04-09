use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Click {
        x: i32,
        y: i32,
        #[serde(default = "default_button")]
        button: String,
    },
    Move {
        x: i32,
        y: i32,
    },
    Type {
        text: String,
    },
    Key {
        combo: String,
    },
    Wait {
        ms: u64,
    },
    Screenshot {
        output: String,
    },
}

fn default_button() -> String {
    "left".to_string()
}

/// Execute a single action. When `live` is false, only prints what would happen.
pub fn execute(action: &Action, live: bool) -> Result<String, String> {
    if !live {
        return Ok(format!("[DRY RUN] Would execute: {:?}", action));
    }

    match action {
        Action::Click { x, y, button } => crate::platform_win::execute_click(*x, *y, button),
        Action::Move { x, y } => crate::platform_win::execute_move(*x, *y),
        Action::Type { text } => crate::platform_win::execute_type(text),
        Action::Key { combo } => crate::platform_win::execute_key(combo),
        Action::Wait { ms } => {
            std::thread::sleep(std::time::Duration::from_millis(*ms));
            Ok(format!("Waited {}ms", ms))
        }
        Action::Screenshot { output } => crate::platform_win::execute_screenshot(output),
    }
}

/// Load a sequence of actions from a JSON file.
pub fn load_script(path: &str) -> Result<Vec<Action>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read script file '{}': {}", path, e))?;
    let actions: Vec<Action> = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid JSON in '{}': {}", path, e))?;
    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Dry-run tests ────────────────────────────────────────────────────────

    #[test]
    fn dry_run_click_prints_would_execute() {
        let a = Action::Click { x: 100, y: 200, button: "left".into() };
        let result = execute(&a, false).unwrap();
        assert!(result.starts_with("[DRY RUN]"), "Expected DRY RUN prefix, got: {}", result);
        assert!(result.contains("Click"), "Expected action name in output: {}", result);
    }

    #[test]
    fn dry_run_move_prints_would_execute() {
        let a = Action::Move { x: 50, y: 75 };
        let result = execute(&a, false).unwrap();
        assert!(result.starts_with("[DRY RUN]"));
        assert!(result.contains("Move"));
    }

    #[test]
    fn dry_run_type_prints_would_execute() {
        let a = Action::Type { text: "Hello World".into() };
        let result = execute(&a, false).unwrap();
        assert!(result.starts_with("[DRY RUN]"));
        assert!(result.contains("Type"));
    }

    #[test]
    fn dry_run_key_prints_would_execute() {
        let a = Action::Key { combo: "ctrl+s".into() };
        let result = execute(&a, false).unwrap();
        assert!(result.starts_with("[DRY RUN]"));
        assert!(result.contains("Key"));
    }

    #[test]
    fn dry_run_wait_prints_would_execute() {
        let a = Action::Wait { ms: 500 };
        let result = execute(&a, false).unwrap();
        assert!(result.starts_with("[DRY RUN]"));
        assert!(result.contains("Wait"));
    }

    #[test]
    fn dry_run_screenshot_prints_would_execute() {
        let a = Action::Screenshot { output: "out.png".into() };
        let result = execute(&a, false).unwrap();
        assert!(result.starts_with("[DRY RUN]"));
        assert!(result.contains("Screenshot"));
    }

    // ── JSON deserialization tests ───────────────────────────────────────────

    #[test]
    fn parse_click_action_from_json() {
        let json = r#"{"type":"click","x":300,"y":400,"button":"right"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::Click { x, y, button } => {
                assert_eq!(x, 300);
                assert_eq!(y, 400);
                assert_eq!(button, "right");
            }
            _ => panic!("Expected Click"),
        }
    }

    #[test]
    fn parse_key_action_from_json() {
        let json = r#"{"type":"key","combo":"alt+tab"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::Key { combo } => assert_eq!(combo, "alt+tab"),
            _ => panic!("Expected Key"),
        }
    }

    #[test]
    fn parse_full_script_from_json() {
        let json = r#"[
            {"type":"move","x":100,"y":100},
            {"type":"click","x":100,"y":100},
            {"type":"type","text":"hello"},
            {"type":"wait","ms":250}
        ]"#;
        let actions: Vec<Action> = serde_json::from_str(json).unwrap();
        assert_eq!(actions.len(), 4);
    }

    #[test]
    fn load_script_missing_file_returns_err() {
        let result = load_script("/nonexistent/path/actions.json");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("Cannot read script file"));
    }

    #[test]
    fn load_script_invalid_json_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "this is not json").unwrap();
        let result = load_script(path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid JSON"));
    }

    #[test]
    fn load_script_valid_file_returns_actions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("actions.json");
        let json = r#"[{"type":"move","x":10,"y":20},{"type":"wait","ms":100}]"#;
        std::fs::write(&path, json).unwrap();
        let actions = load_script(path.to_str().unwrap()).unwrap();
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn dry_run_click_right_button() {
        let action = Action::Click { x: 100, y: 200, button: "right".into() };
        let result = execute(&action, false).unwrap();
        assert!(result.contains("DRY RUN"));
        assert!(result.contains("right"));
    }

    #[test]
    fn dry_run_screenshot() {
        let action = Action::Screenshot { output: "test.png".into() };
        let result = execute(&action, false).unwrap();
        assert!(result.contains("DRY RUN"));
        assert!(result.contains("test.png"));
    }

    #[test]
    fn dry_run_wait() {
        let action = Action::Wait { ms: 500 };
        let result = execute(&action, false).unwrap();
        assert!(result.contains("DRY RUN"));
    }

    #[test]
    fn serde_click_default_button() {
        let json = r#"{"type":"click","x":50,"y":100}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        match action {
            Action::Click { button, .. } => assert_eq!(button, "left"),
            _ => panic!("Expected Click"),
        }
    }

    #[test]
    fn serde_all_action_types() {
        let actions = vec![
            r#"{"type":"click","x":0,"y":0}"#,
            r#"{"type":"move","x":10,"y":20}"#,
            r#"{"type":"type","text":"hello"}"#,
            r#"{"type":"key","combo":"ctrl+s"}"#,
            r#"{"type":"wait","ms":100}"#,
            r#"{"type":"screenshot","output":"out.png"}"#,
        ];
        for json in actions {
            let _: Action = serde_json::from_str(json).unwrap();
        }
    }

    #[test]
    fn serde_invalid_type_errors() {
        let json = r#"{"type":"unknown_action","x":0}"#;
        let result: Result<Action, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn load_script_empty_array() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.json");
        std::fs::write(&path, "[]").unwrap();
        let actions = load_script(path.to_str().unwrap()).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn execute_all_dry_run_succeeds() {
        let actions = vec![
            Action::Click { x: 0, y: 0, button: "left".into() },
            Action::Move { x: 10, y: 20 },
            Action::Type { text: "test".into() },
            Action::Key { combo: "enter".into() },
            Action::Wait { ms: 10 },
            Action::Screenshot { output: "s.png".into() },
        ];
        for action in &actions {
            let result = execute(action, false);
            assert!(result.is_ok(), "Dry run failed for {:?}", action);
        }
    }
}
