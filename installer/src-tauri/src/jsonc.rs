use std::collections::BTreeMap;

use jsonc_parser::cst::{CstInputValue, CstObject, CstRootNode};
use jsonc_parser::ParseOptions;
use serde_json::Value;

fn parse(text: &str) -> Result<CstRootNode, String> {
    let source = if text.trim().is_empty() { "{}" } else { text };
    CstRootNode::parse(source, &ParseOptions::default())
        .map_err(|e| format!("settings.json isn't valid JSON with comments: {e}"))
}

fn object_of(root: &CstRootNode) -> Result<CstObject, String> {
    match root.value() {
        None => Ok(root.object_value_or_set()),
        Some(_) => root
            .object_value()
            .ok_or_else(|| "settings.json doesn't hold a JSON object at its top level.".to_string()),
    }
}

fn input(value: &Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(flag) => CstInputValue::Bool(*flag),
        Value::Number(number) => CstInputValue::Number(number.to_string()),
        Value::String(text) => CstInputValue::String(text.clone()),
        Value::Array(items) => CstInputValue::Array(items.iter().map(input).collect()),
        Value::Object(map) => CstInputValue::Object(map.iter().map(|(k, v)| (k.clone(), input(v))).collect()),
    }
}

pub fn read_values(text: &str, keys: &[&str]) -> Result<BTreeMap<String, Value>, String> {
    let root = parse(text)?;
    let object = object_of(&root)?;
    let mut found = BTreeMap::new();
    for key in keys {
        let value = object
            .get(key)
            .and_then(|prop| prop.value())
            .and_then(|node| node.to_serde_value());
        if let Some(value) = value {
            found.insert((*key).to_string(), value);
        }
    }
    Ok(found)
}

pub fn set_values(text: &str, values: &[(&str, Value)]) -> Result<String, String> {
    let root = parse(text)?;
    let object = object_of(&root)?;
    for (key, value) in values {
        match object.get(key) {
            Some(prop) => prop.set_value(input(value)),
            None => {
                object.append(key, input(value));
            }
        }
    }
    Ok(finish(&root, text))
}

pub fn remove_keys(text: &str, keys: &[&str]) -> Result<String, String> {
    let root = parse(text)?;
    let object = object_of(&root)?;
    for key in keys {
        if let Some(prop) = object.get(key) {
            prop.remove();
        }
    }
    Ok(finish(&root, text))
}

fn finish(root: &CstRootNode, original: &str) -> String {
    let mut out = root.to_string();
    if !original.trim().is_empty() && original.ends_with('\n') && !out.ends_with('\n') {
        out.push_str(if original.ends_with("\r\n") { "\r\n" } else { "\n" });
    }
    if original.trim().is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const KEY_A: &str = "workbench.externalBrowser";
    const KEY_B: &str = "workbench.browser.openLocalhostLinks";
    const WINDOWS_PATH: &str = r"C:\Users\Alex\AppData\Local\Programs\linkgate\linkgate.exe";

    fn values() -> Vec<(&'static str, Value)> {
        vec![(KEY_A, json!(WINDOWS_PATH)), (KEY_B, json!(false))]
    }

    fn reread(text: &str) -> BTreeMap<String, Value> {
        read_values(text, &[KEY_A, KEY_B]).unwrap()
    }

    #[test]
    fn creates_the_object_for_an_empty_or_blank_file() {
        for source in ["", "  \n", "{}"] {
            let out = set_values(source, &values()).unwrap();
            let found = reread(&out);
            assert_eq!(found[KEY_A], json!(WINDOWS_PATH), "source {source:?}");
            assert_eq!(found[KEY_B], json!(false));
            assert_eq!(out.ends_with('\n'), source != "{}", "source {source:?}");
        }
    }

    #[test]
    fn keeps_comments_and_untouched_settings() {
        let source = "{\n  // keep me\n  \"editor.fontSize\": 14, /* inline */\n  \"git.autofetch\": true\n}\n";
        let out = set_values(source, &values()).unwrap();
        assert!(out.contains("// keep me"));
        assert!(out.contains("/* inline */"));
        assert!(out.contains("\"editor.fontSize\": 14"));
        assert_eq!(reread(&out)[KEY_A], json!(WINDOWS_PATH));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn handles_trailing_commas() {
        let source = "{\n  \"a\": 1,\n  \"b\": [1, 2,],\n}\n";
        let out = set_values(source, &values()).unwrap();
        assert_eq!(reread(&out)[KEY_B], json!(false));
        assert!(out.contains("\"b\": [1, 2,]"));
    }

    #[test]
    fn replaces_existing_values_in_place() {
        let source = format!("{{\n  \"{KEY_A}\": \"edge\",\n  \"x\": 1,\n  \"{KEY_B}\": true\n}}\n");
        let out = set_values(&source, &values()).unwrap();
        let found = reread(&out);
        assert_eq!(found[KEY_A], json!(WINDOWS_PATH));
        assert_eq!(found[KEY_B], json!(false));
        assert_eq!(out.matches(KEY_A).count(), 1);
        assert!(out.find(KEY_A).unwrap() < out.find("\"x\"").unwrap());
    }

    #[test]
    fn keeps_crlf_line_endings() {
        let source = "{\r\n  \"a\": 1\r\n}\r\n";
        let out = set_values(source, &values()).unwrap();
        assert!(!out.replace("\r\n", "").contains('\n'), "bare LF in {out:?}");
        assert!(out.ends_with("\r\n"));
        assert_eq!(reread(&out)[KEY_A], json!(WINDOWS_PATH));
    }

    #[test]
    fn handles_a_file_holding_only_comments() {
        let out = set_values("// settings\n", &values()).unwrap();
        assert!(out.contains("// settings"));
        assert_eq!(reread(&out)[KEY_A], json!(WINDOWS_PATH));
    }

    #[test]
    fn removes_only_the_named_keys() {
        let source = set_values("{\n  // c\n  \"keep\": 1\n}\n", &values()).unwrap();
        let out = remove_keys(&source, &[KEY_A, KEY_B]).unwrap();
        assert!(reread(&out).is_empty());
        assert!(out.contains("// c"));
        assert!(out.contains("\"keep\": 1"));
        assert!(CstRootNode::parse(&out, &ParseOptions::default()).is_ok());
    }

    #[test]
    fn removing_missing_keys_changes_nothing() {
        let source = "{\n  \"a\": 1\n}\n";
        assert_eq!(remove_keys(source, &[KEY_A]).unwrap(), source);
    }

    #[test]
    fn round_trips_previous_values_of_any_type() {
        let source = "{\n  \"workbench.externalBrowser\": \"edge\"\n}\n";
        let previous = read_values(source, &[KEY_A, KEY_B]).unwrap();
        assert_eq!(previous.len(), 1);
        let out = set_values(source, &values()).unwrap();
        let restored = set_values(&out, &[(KEY_A, previous[KEY_A].clone())]).unwrap();
        let restored = remove_keys(&restored, &[KEY_B]).unwrap();
        assert_eq!(reread(&restored), previous);
    }

    #[test]
    fn rejects_files_that_are_not_objects_or_not_json() {
        assert!(set_values("[1, 2]", &values()).is_err());
        assert!(set_values("{ \"a\": ", &values()).is_err());
    }
}
