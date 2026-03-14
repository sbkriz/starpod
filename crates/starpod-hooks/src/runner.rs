//! Hook execution engine — runs matched hooks with timeout and cancellation support.

use std::collections::HashMap;

use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::callback::HookCallbackMatcher;
use crate::event::HookEvent;
use crate::input::{BaseHookInput, HookInput};
use crate::output::HookOutput;

/// A registry of hooks keyed by event type.
///
/// Wraps a `HashMap<HookEvent, Vec<HookCallbackMatcher>>` and provides
/// methods to run hooks for specific events.
///
/// # Example
///
/// ```
/// use starpod_hooks::{HookRegistry, HookEvent, HookCallbackMatcher, hook_fn, HookOutput};
///
/// let mut registry = HookRegistry::new();
/// registry.register(HookEvent::PostToolUse, vec![
///     HookCallbackMatcher::new(vec![
///         hook_fn(|_input, _id, _cancel| async move {
///             Ok(HookOutput::default())
///         }),
///     ]).with_matcher("Bash"),
/// ]);
///
/// assert!(registry.has_hooks(&HookEvent::PostToolUse));
/// assert!(!registry.has_hooks(&HookEvent::PreToolUse));
/// ```
#[derive(Debug, Clone, Default)]
pub struct HookRegistry {
    hooks: HashMap<HookEvent, Vec<HookCallbackMatcher>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry from an existing HashMap.
    pub fn from_map(hooks: HashMap<HookEvent, Vec<HookCallbackMatcher>>) -> Self {
        Self { hooks }
    }

    /// Register matchers for a hook event.
    pub fn register(&mut self, event: HookEvent, matchers: Vec<HookCallbackMatcher>) {
        self.hooks.insert(event, matchers);
    }

    /// Check if any hooks are registered for the given event.
    pub fn has_hooks(&self, event: &HookEvent) -> bool {
        self.hooks.get(event).is_some_and(|m| !m.is_empty())
    }

    /// Get the matchers for a given event.
    pub fn get(&self, event: &HookEvent) -> Option<&Vec<HookCallbackMatcher>> {
        self.hooks.get(event)
    }

    /// Consume the registry and return the inner HashMap.
    pub fn into_map(self) -> HashMap<HookEvent, Vec<HookCallbackMatcher>> {
        self.hooks
    }

    /// Run all matching hooks for PostToolUse.
    ///
    /// Hooks are fire-and-forget: errors are logged but do not propagate.
    pub async fn run_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        tool_response: &serde_json::Value,
        tool_use_id: &str,
        session_id: &str,
        cwd: &str,
    ) {
        if let Some(matchers) = self.hooks.get(&HookEvent::PostToolUse) {
            run_hooks_for_tool(
                matchers,
                HookEvent::PostToolUse,
                tool_name,
                tool_input,
                Some(tool_response),
                tool_use_id,
                session_id,
                cwd,
            )
            .await;
        }
    }

    /// Run all matching hooks for PreToolUse.
    ///
    /// Returns the merged [`HookOutput`] from all matching hooks,
    /// or `None` if no hooks matched.
    pub async fn run_pre_tool_use(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        tool_use_id: &str,
        session_id: &str,
        cwd: &str,
    ) -> Option<HookOutput> {
        let matchers = self.hooks.get(&HookEvent::PreToolUse)?;
        run_hooks_for_tool_with_output(
            matchers,
            HookEvent::PreToolUse,
            tool_name,
            tool_input,
            None,
            tool_use_id,
            session_id,
            cwd,
        )
        .await
    }

    /// Run hooks for a generic (non-tool) event.
    ///
    /// Fires all registered hooks for the event. Errors are logged.
    pub async fn run_event(&self, event: &HookEvent, input: HookInput) {
        if let Some(matchers) = self.hooks.get(event) {
            run_generic_hooks(matchers, input).await;
        }
    }
}

/// Run hooks that match a tool name, fire-and-forget style.
async fn run_hooks_for_tool(
    matchers: &[HookCallbackMatcher],
    event: HookEvent,
    tool_name: &str,
    tool_input: &serde_json::Value,
    tool_response: Option<&serde_json::Value>,
    tool_use_id: &str,
    session_id: &str,
    cwd: &str,
) {
    for matcher in matchers {
        if !matcher.matches(tool_name).unwrap_or(false) {
            continue;
        }

        let input = build_tool_hook_input(
            &event,
            tool_name,
            tool_input,
            tool_response,
            tool_use_id,
            session_id,
            cwd,
        );

        let cancel = CancellationToken::new();
        let timeout_secs = matcher.timeout;

        for hook in &matcher.hooks {
            let fut = hook(input.clone(), Some(tool_use_id.to_string()), cancel.clone());

            if let Some(secs) = timeout_secs {
                match tokio::time::timeout(std::time::Duration::from_secs(secs), fut).await {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => warn!("{} hook error: {}", event, e),
                    Err(_) => warn!("{} hook timed out after {}s", event, secs),
                }
            } else if let Err(e) = fut.await {
                warn!("{} hook error: {}", event, e);
            }
        }
    }
}

/// Run hooks that match a tool name, collecting the last sync output.
async fn run_hooks_for_tool_with_output(
    matchers: &[HookCallbackMatcher],
    event: HookEvent,
    tool_name: &str,
    tool_input: &serde_json::Value,
    tool_response: Option<&serde_json::Value>,
    tool_use_id: &str,
    session_id: &str,
    cwd: &str,
) -> Option<HookOutput> {
    let mut last_output: Option<HookOutput> = None;

    for matcher in matchers {
        if !matcher.matches(tool_name).unwrap_or(false) {
            continue;
        }

        let input = build_tool_hook_input(
            &event,
            tool_name,
            tool_input,
            tool_response,
            tool_use_id,
            session_id,
            cwd,
        );

        let cancel = CancellationToken::new();
        let timeout_secs = matcher.timeout;

        for hook in &matcher.hooks {
            let fut = hook(input.clone(), Some(tool_use_id.to_string()), cancel.clone());

            let result = if let Some(secs) = timeout_secs {
                match tokio::time::timeout(std::time::Duration::from_secs(secs), fut).await {
                    Ok(r) => r,
                    Err(_) => {
                        warn!("{} hook timed out after {}s", event, secs);
                        continue;
                    }
                }
            } else {
                fut.await
            };

            match result {
                Ok(output) => last_output = Some(output),
                Err(e) => warn!("{} hook error: {}", event, e),
            }
        }
    }

    last_output
}

/// Run hooks for a non-tool event (no regex matching on tool name).
async fn run_generic_hooks(matchers: &[HookCallbackMatcher], input: HookInput) {
    let cancel = CancellationToken::new();

    for matcher in matchers {
        let timeout_secs = matcher.timeout;

        for hook in &matcher.hooks {
            let fut = hook(input.clone(), None, cancel.clone());

            if let Some(secs) = timeout_secs {
                match tokio::time::timeout(std::time::Duration::from_secs(secs), fut).await {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => warn!("Hook error: {}", e),
                    Err(_) => warn!("Hook timed out after {}s", secs),
                }
            } else if let Err(e) = fut.await {
                warn!("Hook error: {}", e);
            }
        }
    }
}

/// Build a HookInput for tool-related events.
fn build_tool_hook_input(
    event: &HookEvent,
    tool_name: &str,
    tool_input: &serde_json::Value,
    tool_response: Option<&serde_json::Value>,
    tool_use_id: &str,
    session_id: &str,
    cwd: &str,
) -> HookInput {
    let base = BaseHookInput {
        session_id: session_id.to_string(),
        transcript_path: String::new(),
        cwd: cwd.to_string(),
        permission_mode: None,
        agent_id: None,
        agent_type: None,
    };

    match event {
        HookEvent::PostToolUse => HookInput::PostToolUse {
            base,
            tool_name: tool_name.to_string(),
            tool_input: tool_input.clone(),
            tool_response: tool_response.cloned().unwrap_or_default(),
            tool_use_id: tool_use_id.to_string(),
        },
        HookEvent::PreToolUse => HookInput::PreToolUse {
            base,
            tool_name: tool_name.to_string(),
            tool_input: tool_input.clone(),
            tool_use_id: tool_use_id.to_string(),
        },
        HookEvent::PostToolUseFailure => HookInput::PostToolUseFailure {
            base,
            tool_name: tool_name.to_string(),
            tool_input: tool_input.clone(),
            tool_use_id: tool_use_id.to_string(),
            error: tool_response
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            is_interrupt: None,
        },
        // For other events, fallback to PostToolUse shape (shouldn't happen
        // in practice since callers use the right event).
        _ => HookInput::PostToolUse {
            base,
            tool_name: tool_name.to_string(),
            tool_input: tool_input.clone(),
            tool_response: tool_response.cloned().unwrap_or_default(),
            tool_use_id: tool_use_id.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callback::{hook_fn, HookCallbackMatcher};
    use crate::output::HookOutput;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn registry_new_is_empty() {
        let reg = HookRegistry::new();
        assert!(!reg.has_hooks(&HookEvent::PostToolUse));
        assert!(!reg.has_hooks(&HookEvent::PreToolUse));
    }

    #[test]
    fn registry_register_and_has_hooks() {
        let mut reg = HookRegistry::new();
        reg.register(
            HookEvent::PostToolUse,
            vec![HookCallbackMatcher::new(vec![hook_fn(
                |_i, _id, _c| async { Ok(HookOutput::default()) },
            )])],
        );
        assert!(reg.has_hooks(&HookEvent::PostToolUse));
        assert!(!reg.has_hooks(&HookEvent::PreToolUse));
    }

    #[test]
    fn registry_from_map_and_into_map() {
        let mut map = HashMap::new();
        map.insert(
            HookEvent::Stop,
            vec![HookCallbackMatcher::new(vec![hook_fn(
                |_i, _id, _c| async { Ok(HookOutput::default()) },
            )])],
        );
        let reg = HookRegistry::from_map(map);
        assert!(reg.has_hooks(&HookEvent::Stop));
        let map = reg.into_map();
        assert!(map.contains_key(&HookEvent::Stop));
    }

    #[test]
    fn registry_get_returns_matchers() {
        let mut reg = HookRegistry::new();
        let matcher = HookCallbackMatcher::new(vec![hook_fn(
            |_i, _id, _c| async { Ok(HookOutput::default()) },
        )])
        .with_matcher("Bash");
        reg.register(HookEvent::PostToolUse, vec![matcher]);
        let matchers = reg.get(&HookEvent::PostToolUse).unwrap();
        assert_eq!(matchers.len(), 1);
        assert_eq!(matchers[0].matcher.as_deref(), Some("Bash"));
    }

    #[tokio::test]
    async fn run_post_tool_use_fires_matching_hooks() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut reg = HookRegistry::new();
        reg.register(
            HookEvent::PostToolUse,
            vec![HookCallbackMatcher::new(vec![hook_fn(move |_i, _id, _c| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(HookOutput::default())
                }
            })])
            .with_matcher("Bash")],
        );

        // Should fire for Bash
        reg.run_post_tool_use(
            "Bash",
            &serde_json::json!({"command": "ls"}),
            &serde_json::json!("output"),
            "tu-1",
            "sess-1",
            "/tmp",
        )
        .await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Should NOT fire for Read (doesn't match "Bash" regex)
        reg.run_post_tool_use(
            "Read",
            &serde_json::json!({}),
            &serde_json::json!(""),
            "tu-2",
            "sess-1",
            "/tmp",
        )
        .await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn run_post_tool_use_no_matcher_fires_for_all() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut reg = HookRegistry::new();
        reg.register(
            HookEvent::PostToolUse,
            vec![HookCallbackMatcher::new(vec![hook_fn(move |_i, _id, _c| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(HookOutput::default())
                }
            })])],
        );

        reg.run_post_tool_use("Bash", &serde_json::json!({}), &serde_json::json!(""), "tu-1", "s", "/tmp").await;
        reg.run_post_tool_use("Read", &serde_json::json!({}), &serde_json::json!(""), "tu-2", "s", "/tmp").await;
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn run_pre_tool_use_returns_output() {
        let mut reg = HookRegistry::new();
        reg.register(
            HookEvent::PreToolUse,
            vec![HookCallbackMatcher::new(vec![hook_fn(
                |_i, _id, _c| async {
                    Ok(HookOutput::Sync(crate::output::SyncHookOutput {
                        decision: Some(crate::output::HookDecision::Block),
                        reason: Some("blocked".into()),
                        ..Default::default()
                    }))
                },
            )])],
        );

        let output = reg
            .run_pre_tool_use("Bash", &serde_json::json!({}), "tu-1", "s", "/tmp")
            .await;
        assert!(output.is_some());
        match output.unwrap() {
            HookOutput::Sync(sync) => {
                assert_eq!(sync.decision, Some(crate::output::HookDecision::Block));
            }
            _ => panic!("expected sync output"),
        }
    }

    #[tokio::test]
    async fn run_pre_tool_use_returns_none_when_no_hooks() {
        let reg = HookRegistry::new();
        let output = reg
            .run_pre_tool_use("Bash", &serde_json::json!({}), "tu-1", "s", "/tmp")
            .await;
        assert!(output.is_none());
    }

    #[tokio::test]
    async fn run_event_fires_generic_hooks() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut reg = HookRegistry::new();
        reg.register(
            HookEvent::SessionStart,
            vec![HookCallbackMatcher::new(vec![hook_fn(move |_i, _id, _c| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(HookOutput::default())
                }
            })])],
        );

        let input = HookInput::SessionStart {
            base: BaseHookInput {
                session_id: "s".into(),
                transcript_path: String::new(),
                cwd: "/tmp".into(),
                permission_mode: None,
                agent_id: None,
                agent_type: None,
            },
            source: crate::input::SessionStartSource::Startup,
            model: None,
        };

        reg.run_event(&HookEvent::SessionStart, input).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn hook_error_is_logged_not_propagated() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut reg = HookRegistry::new();
        reg.register(
            HookEvent::PostToolUse,
            vec![HookCallbackMatcher::new(vec![
                // First hook errors
                hook_fn(|_i, _id, _c| async {
                    Err(crate::error::HookError::CallbackFailed("oops".into()))
                }),
                // Second hook should still run
                hook_fn(move |_i, _id, _c| {
                    let counter = counter_clone.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Ok(HookOutput::default())
                    }
                }),
            ])],
        );

        reg.run_post_tool_use("Bash", &serde_json::json!({}), &serde_json::json!(""), "tu-1", "s", "/tmp")
            .await;
        // Second hook should have fired despite the first one erroring
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn hook_timeout_is_enforced() {
        let mut reg = HookRegistry::new();
        reg.register(
            HookEvent::PostToolUse,
            vec![HookCallbackMatcher::new(vec![hook_fn(|_i, _id, _c| async {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                Ok(HookOutput::default())
            })])
            .with_timeout(1)],  // 1 second timeout
        );

        let start = std::time::Instant::now();
        reg.run_post_tool_use("Bash", &serde_json::json!({}), &serde_json::json!(""), "tu-1", "s", "/tmp")
            .await;
        let elapsed = start.elapsed();
        // Should complete in ~1s, not 10s
        assert!(elapsed.as_secs() < 3, "hook should have timed out, took {:?}", elapsed);
    }

    #[tokio::test]
    async fn multiple_matchers_all_fire() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = counter.clone();
        let c2 = counter.clone();

        let mut reg = HookRegistry::new();
        reg.register(
            HookEvent::PostToolUse,
            vec![
                HookCallbackMatcher::new(vec![hook_fn(move |_i, _id, _c| {
                    let c = c1.clone();
                    async move { c.fetch_add(1, Ordering::SeqCst); Ok(HookOutput::default()) }
                })]).with_matcher("Bash"),
                HookCallbackMatcher::new(vec![hook_fn(move |_i, _id, _c| {
                    let c = c2.clone();
                    async move { c.fetch_add(10, Ordering::SeqCst); Ok(HookOutput::default()) }
                })]).with_matcher("Bash|Read"),
            ],
        );

        reg.run_post_tool_use("Bash", &serde_json::json!({}), &serde_json::json!(""), "tu-1", "s", "/tmp").await;
        assert_eq!(counter.load(Ordering::SeqCst), 11); // both matchers fired
    }

    #[tokio::test]
    async fn run_post_tool_use_noop_when_no_hooks_registered() {
        let reg = HookRegistry::new();
        // Should not panic or error
        reg.run_post_tool_use("Bash", &serde_json::json!({}), &serde_json::json!(""), "tu-1", "s", "/tmp").await;
    }

    #[tokio::test]
    async fn run_event_noop_when_no_hooks_registered() {
        let reg = HookRegistry::new();
        let input = HookInput::SessionEnd {
            base: BaseHookInput {
                session_id: "s".into(),
                transcript_path: String::new(),
                cwd: "/tmp".into(),
                permission_mode: None,
                agent_id: None,
                agent_type: None,
            },
            reason: "user closed".into(),
        };
        // Should not panic
        reg.run_event(&HookEvent::SessionEnd, input).await;
    }

    #[tokio::test]
    async fn hook_receives_correct_input_fields() {
        let received_tool = Arc::new(std::sync::Mutex::new(String::new()));
        let received_clone = received_tool.clone();

        let mut reg = HookRegistry::new();
        reg.register(
            HookEvent::PostToolUse,
            vec![HookCallbackMatcher::new(vec![hook_fn(move |input, tool_use_id, _c| {
                let received = received_clone.clone();
                async move {
                    if let HookInput::PostToolUse { tool_name, base, .. } = &input {
                        *received.lock().unwrap() = format!("{}:{}:{}", tool_name, base.session_id, tool_use_id.unwrap_or_default());
                    }
                    Ok(HookOutput::default())
                }
            })])],
        );

        reg.run_post_tool_use(
            "Write",
            &serde_json::json!({"file_path": "/tmp/test"}),
            &serde_json::json!("ok"),
            "tu-42",
            "sess-abc",
            "/projects/foo",
        ).await;

        assert_eq!(*received_tool.lock().unwrap(), "Write:sess-abc:tu-42");
    }

    #[tokio::test]
    async fn pre_tool_use_non_matching_returns_none() {
        let mut reg = HookRegistry::new();
        reg.register(
            HookEvent::PreToolUse,
            vec![HookCallbackMatcher::new(vec![hook_fn(
                |_i, _id, _c| async {
                    Ok(HookOutput::Sync(crate::output::SyncHookOutput {
                        decision: Some(crate::output::HookDecision::Block),
                        ..Default::default()
                    }))
                },
            )]).with_matcher("Write")],
        );

        // "Bash" doesn't match "Write" regex
        let output = reg.run_pre_tool_use("Bash", &serde_json::json!({}), "tu-1", "s", "/tmp").await;
        assert!(output.is_none());
    }

    #[tokio::test]
    async fn has_hooks_false_for_empty_matchers_vec() {
        let mut reg = HookRegistry::new();
        reg.register(HookEvent::Stop, vec![]); // registered but empty
        assert!(!reg.has_hooks(&HookEvent::Stop));
    }
}
