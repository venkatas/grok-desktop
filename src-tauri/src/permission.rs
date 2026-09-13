use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionOption {
    #[serde(rename = "optionId")]
    pub option_id: String,
    pub name: String,
    pub kind: String,
}

pub fn display_label(kind: &str, name: &str) -> String {
    match kind {
        "allow_once" => "Allow once".into(),
        "allow_always" => "Allow session".into(),
        "reject_once" | "reject_always" => "Deny".into(),
        _ => name.to_string(),
    }
}

pub fn unanswered_deny_result() -> Value {
    json!({ "outcome": { "outcome": "cancelled" } })
}

pub fn selected_result(option_id: &str) -> Value {
    json!({ "outcome": { "outcome": "selected", "optionId": option_id } })
}

pub fn ui_options(options: &[PermissionOption]) -> Vec<PermissionOption> {
    options
        .iter()
        .map(|o| PermissionOption {
            option_id: o.option_id.clone(),
            name: display_label(&o.kind, &o.name),
            kind: o.kind.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unanswered_is_cancelled_not_allow() {
        let v = unanswered_deny_result();
        assert_eq!(v["outcome"]["outcome"], "cancelled");
        assert!(v["outcome"].get("optionId").is_none());
    }

    #[test]
    fn maps_allow_always_to_allow_session() {
        assert_eq!(display_label("allow_always", "Always"), "Allow session");
        assert_eq!(display_label("allow_once", "Once"), "Allow once");
    }

    #[test]
    fn keeps_unknown_option_names() {
        assert_eq!(display_label("custom", "Do it"), "Do it");
    }
}
