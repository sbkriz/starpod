//! Permission types shared between hooks and the broader permission system.

use serde::{Deserialize, Serialize};

/// A permission update for a specific tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionUpdate {
    pub tool: String,
    pub permission: PermissionLevel,
}

/// Permission level for a tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    Allow,
    Deny,
    Ask,
}

/// Permission decision returned by PreToolUse hooks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_update_serde() {
        let update = PermissionUpdate {
            tool: "Bash".into(),
            permission: PermissionLevel::Allow,
        };
        let json = serde_json::to_string(&update).unwrap();
        let back: PermissionUpdate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tool, "Bash");
        assert_eq!(back.permission, PermissionLevel::Allow);
    }

    #[test]
    fn permission_level_all_variants() {
        for (variant, expected) in [
            (PermissionLevel::Allow, "\"allow\""),
            (PermissionLevel::Deny, "\"deny\""),
            (PermissionLevel::Ask, "\"ask\""),
        ] {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected);
        }
    }

    #[test]
    fn permission_decision_all_variants() {
        for (variant, expected) in [
            (PermissionDecision::Allow, "\"allow\""),
            (PermissionDecision::Deny, "\"deny\""),
            (PermissionDecision::Ask, "\"ask\""),
        ] {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected);
        }
    }
}
