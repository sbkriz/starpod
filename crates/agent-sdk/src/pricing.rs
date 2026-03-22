//! Model pricing registry with embedded defaults and runtime overrides.
//!
//! The registry ships with hardcoded default rates (from `defaults/pricing.toml`)
//! and supports overlaying a user-provided TOML file at runtime. This means
//! pricing updates don't require recompilation.
//!
//! # Example
//!
//! ```rust
//! use agent_sdk::pricing::PricingRegistry;
//!
//! let registry = PricingRegistry::with_defaults();
//! let rates = registry.get("anthropic", "claude-sonnet-4-5").unwrap();
//! assert!(rates.input_per_million > 0.0);
//! ```

use std::collections::HashMap;

use serde::Deserialize;
use tracing::debug;

use crate::provider::CostRates;

/// Embedded default pricing (compiled into the binary).
const DEFAULTS_TOML: &str = include_str!("defaults/pricing.toml");

// ── TOML serde types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct PricingFile {
    #[serde(flatten)]
    providers: HashMap<String, ProviderPricing>,
}

#[derive(Debug, Deserialize)]
struct ProviderPricing {
    #[serde(default)]
    cache_read_multiplier: Option<f64>,
    #[serde(default)]
    cache_creation_multiplier: Option<f64>,
    #[serde(default)]
    models: HashMap<String, ModelPricing>,
}

#[derive(Debug, Deserialize)]
struct ModelPricing {
    input: f64,
    output: f64,
    #[serde(default)]
    cache_read_multiplier: Option<f64>,
    #[serde(default)]
    cache_creation_multiplier: Option<f64>,
}

// ── Registry ──────────────────────────────────────────────────────────

/// Composite key: `"provider::model"`.
type PricingKey = String;

fn make_key(provider: &str, model: &str) -> PricingKey {
    format!("{provider}::{model}")
}

/// Thread-safe registry of model pricing rates.
///
/// Lookup order:
/// 1. Exact match on `"provider::model"`
/// 2. Fuzzy match — any registered model whose name is a substring of the
///    query (or vice-versa), scoped to the same provider
/// 3. Provider-level default entry (`"provider::_default"`)
#[derive(Debug, Clone)]
pub struct PricingRegistry {
    entries: HashMap<PricingKey, CostRates>,
}

impl PricingRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Create a registry pre-loaded with the embedded default pricing.
    pub fn with_defaults() -> Self {
        Self::from_toml(DEFAULTS_TOML).expect("embedded pricing.toml must be valid")
    }

    /// Parse a TOML string into a registry.
    pub fn from_toml(toml_str: &str) -> Result<Self, String> {
        let file: PricingFile =
            toml::from_str(toml_str).map_err(|e| format!("pricing TOML parse error: {e}"))?;

        let mut entries = HashMap::new();

        for (provider, pp) in &file.providers {
            // Provider-level cache multipliers (inherited by all models unless overridden).
            let prov_cache_read = pp.cache_read_multiplier;
            let prov_cache_create = pp.cache_creation_multiplier;

            for (model, mp) in &pp.models {
                let rates = CostRates {
                    input_per_million: mp.input,
                    output_per_million: mp.output,
                    cache_read_multiplier: mp.cache_read_multiplier.or(prov_cache_read),
                    cache_creation_multiplier: mp.cache_creation_multiplier.or(prov_cache_create),
                };
                entries.insert(make_key(provider, model), rates);
            }

            // Store a provider-level default with zero rates (cache multipliers only)
            // so fuzzy lookups for unknown models still get correct cache pricing.
            if prov_cache_read.is_some() || prov_cache_create.is_some() {
                entries.insert(
                    make_key(provider, "_default"),
                    CostRates {
                        input_per_million: 0.0,
                        output_per_million: 0.0,
                        cache_read_multiplier: prov_cache_read,
                        cache_creation_multiplier: prov_cache_create,
                    },
                );
            }
        }

        Ok(Self { entries })
    }

    /// Insert or replace a single entry.
    pub fn insert(&mut self, provider: &str, model: &str, rates: CostRates) {
        self.entries.insert(make_key(provider, model), rates);
    }

    /// Merge another registry on top (overrides win).
    pub fn merge(&mut self, other: Self) {
        for (key, rates) in other.entries {
            self.entries.insert(key, rates);
        }
    }

    /// Exact-match lookup.
    pub fn get(&self, provider: &str, model: &str) -> Option<&CostRates> {
        self.entries.get(&make_key(provider, model))
    }

    /// Fuzzy lookup: tries exact match first, then substring matching
    /// against all models for the given provider, then the provider default.
    pub fn get_fuzzy(&self, provider: &str, model: &str) -> Option<&CostRates> {
        // 1. Exact match
        if let Some(rates) = self.get(provider, model) {
            return Some(rates);
        }

        let prefix = format!("{provider}::");

        // 2. Substring match (either direction)
        let mut best: Option<(&str, &CostRates)> = None;
        for (key, rates) in &self.entries {
            if let Some(registered_model) = key.strip_prefix(&prefix) {
                if registered_model == "_default" {
                    continue;
                }
                // Check if either is a substring of the other
                if model.contains(registered_model) || registered_model.contains(model) {
                    // Prefer the longest registered model name (most specific match)
                    let dominated = best
                        .map(|(prev, _)| registered_model.len() > prev.len())
                        .unwrap_or(true);
                    if dominated {
                        best = Some((registered_model, rates));
                    }
                }
            }
        }
        if let Some((matched, rates)) = best {
            debug!(provider, model, matched, "fuzzy pricing match");
            return Some(rates);
        }

        // 3. Provider default (cache multipliers only)
        self.entries.get(&make_key(provider, "_default"))
    }

    /// Number of entries (including provider defaults).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for PricingRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_load_successfully() {
        let reg = PricingRegistry::with_defaults();
        assert!(!reg.is_empty());
    }

    #[test]
    fn exact_match() {
        let reg = PricingRegistry::with_defaults();
        let rates = reg.get("anthropic", "claude-sonnet-4-5").unwrap();
        assert!((rates.input_per_million - 3.0).abs() < 1e-9);
        assert!((rates.output_per_million - 15.0).abs() < 1e-9);
        assert!((rates.cache_read_multiplier.unwrap() - 0.1).abs() < 1e-9);
        assert!((rates.cache_creation_multiplier.unwrap() - 1.25).abs() < 1e-9);
    }

    #[test]
    fn fuzzy_match_longer_model_id() {
        let reg = PricingRegistry::with_defaults();
        // A versioned model ID should fuzzy-match the base name
        let rates = reg.get_fuzzy("anthropic", "claude-sonnet-4-5-20250514").unwrap();
        assert!((rates.input_per_million - 3.0).abs() < 1e-9);
    }

    #[test]
    fn fuzzy_match_picks_most_specific() {
        let mut reg = PricingRegistry::new();
        reg.insert("test", "claude-sonnet", CostRates {
            input_per_million: 1.0,
            output_per_million: 5.0,
            cache_read_multiplier: None,
            cache_creation_multiplier: None,
        });
        reg.insert("test", "claude-sonnet-4-5", CostRates {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cache_read_multiplier: None,
            cache_creation_multiplier: None,
        });
        // Should pick the more specific "claude-sonnet-4-5" over "claude-sonnet"
        let rates = reg.get_fuzzy("test", "claude-sonnet-4-5-20250514").unwrap();
        assert!((rates.input_per_million - 3.0).abs() < 1e-9);
    }

    #[test]
    fn provider_default_returns_cache_multipliers() {
        let reg = PricingRegistry::with_defaults();
        // Unknown model, but provider default should have cache multipliers
        let rates = reg.get_fuzzy("anthropic", "claude-unknown-model-99").unwrap();
        assert!((rates.cache_read_multiplier.unwrap() - 0.1).abs() < 1e-9);
    }

    #[test]
    fn merge_overrides() {
        let mut base = PricingRegistry::with_defaults();
        let mut overrides = PricingRegistry::new();
        overrides.insert("anthropic", "claude-sonnet-4-5", CostRates {
            input_per_million: 99.0,
            output_per_million: 99.0,
            cache_read_multiplier: Some(0.5),
            cache_creation_multiplier: Some(2.0),
        });
        base.merge(overrides);

        let rates = base.get("anthropic", "claude-sonnet-4-5").unwrap();
        assert!((rates.input_per_million - 99.0).abs() < 1e-9);
    }

    #[test]
    fn openai_cache_rates() {
        let reg = PricingRegistry::with_defaults();
        let rates = reg.get("openai", "gpt-4o").unwrap();
        assert!((rates.cache_read_multiplier.unwrap() - 0.5).abs() < 1e-9);
        assert!((rates.cache_creation_multiplier.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn gemini_cache_rates() {
        let reg = PricingRegistry::with_defaults();
        let rates = reg.get_fuzzy("gemini", "gemini-2-5-flash").unwrap();
        assert!((rates.cache_read_multiplier.unwrap() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn from_toml_custom() {
        let toml = r#"
[custom]
cache_read_multiplier = 0.3

[custom.models.my-model]
input = 5.0
output = 20.0
"#;
        let reg = PricingRegistry::from_toml(toml).unwrap();
        let rates = reg.get("custom", "my-model").unwrap();
        assert!((rates.input_per_million - 5.0).abs() < 1e-9);
        assert!((rates.cache_read_multiplier.unwrap() - 0.3).abs() < 1e-9);
        assert!(rates.cache_creation_multiplier.is_none());
    }

    #[test]
    fn per_model_cache_override() {
        let toml = r#"
[prov]
cache_read_multiplier = 0.1
cache_creation_multiplier = 1.25

[prov.models.special]
input = 10.0
output = 50.0
cache_read_multiplier = 0.05
"#;
        let reg = PricingRegistry::from_toml(toml).unwrap();
        let rates = reg.get("prov", "special").unwrap();
        // Per-model override wins for cache_read
        assert!((rates.cache_read_multiplier.unwrap() - 0.05).abs() < 1e-9);
        // Falls back to provider-level for cache_creation
        assert!((rates.cache_creation_multiplier.unwrap() - 1.25).abs() < 1e-9);
    }

    #[test]
    fn empty_provider_no_panic() {
        let toml = r#"
[empty]
"#;
        let reg = PricingRegistry::from_toml(toml).unwrap();
        assert!(reg.get("empty", "anything").is_none());
        assert!(reg.get_fuzzy("empty", "anything").is_none());
    }
}
