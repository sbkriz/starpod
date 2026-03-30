//! Default allowed-host suggestions for well-known API key names.

/// Returns suggested `allowed_hosts` for well-known secret key names.
///
/// Used by `VaultSet` to auto-suggest host bindings when the user doesn't
/// specify them. Returns `None` for unrecognised keys (meaning unrestricted).
///
/// ```
/// use starpod_vault::known_hosts::default_hosts_for_key;
///
/// let hosts = default_hosts_for_key("OPENAI_API_KEY").unwrap();
/// assert_eq!(hosts, vec!["api.openai.com"]);
///
/// assert!(default_hosts_for_key("MY_CUSTOM_VAR").is_none());
/// ```
pub fn default_hosts_for_key(key: &str) -> Option<Vec<String>> {
    match key.to_uppercase().as_str() {
        // LLM providers
        "ANTHROPIC_API_KEY" => Some(vec!["api.anthropic.com".into()]),
        "OPENAI_API_KEY" => Some(vec!["api.openai.com".into()]),
        "GEMINI_API_KEY" => Some(vec!["generativelanguage.googleapis.com".into()]),
        "GROQ_API_KEY" => Some(vec!["api.groq.com".into()]),
        "DEEPSEEK_API_KEY" => Some(vec!["api.deepseek.com".into()]),
        "OPENROUTER_API_KEY" => Some(vec!["openrouter.ai".into()]),

        // Services
        "BRAVE_API_KEY" => Some(vec!["api.search.brave.com".into()]),
        "TELEGRAM_BOT_TOKEN" => Some(vec!["api.telegram.org".into()]),

        // Developer tools
        "GITHUB_TOKEN" | "GH_TOKEN" => Some(vec!["api.github.com".into()]),
        "STRIPE_SECRET_KEY" | "STRIPE_API_KEY" => Some(vec!["api.stripe.com".into()]),
        "SENDGRID_API_KEY" => Some(vec!["api.sendgrid.com".into()]),
        "SLACK_BOT_TOKEN" | "SLACK_TOKEN" => Some(vec!["slack.com".into()]),
        "AWS_SECRET_ACCESS_KEY" => Some(vec!["*.amazonaws.com".into()]),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_keys_return_hosts() {
        assert!(default_hosts_for_key("ANTHROPIC_API_KEY").is_some());
        assert!(default_hosts_for_key("OPENAI_API_KEY").is_some());
        assert!(default_hosts_for_key("TELEGRAM_BOT_TOKEN").is_some());
        assert!(default_hosts_for_key("GITHUB_TOKEN").is_some());
        assert!(default_hosts_for_key("GH_TOKEN").is_some());
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(
            default_hosts_for_key("openai_api_key"),
            default_hosts_for_key("OPENAI_API_KEY"),
        );
    }

    #[test]
    fn unknown_keys_return_none() {
        assert!(default_hosts_for_key("MY_VAR").is_none());
        assert!(default_hosts_for_key("DB_PASSWORD").is_none());
    }
}
