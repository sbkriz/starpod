//! # starpod-hooks — Lifecycle hook system for Starpod
//!
//! This crate provides the hook infrastructure for the Starpod AI assistant
//! platform. It defines hook events, input/output types, callback mechanisms,
//! and an execution engine with timeout and cancellation support.
//!
//! ## Architecture
//!
//! Hooks are lifecycle callbacks that fire at specific points during agent
//! execution. They can observe events (fire-and-forget) or control behavior
//! (blocking with decisions).
//!
//! The system is built around these concepts:
//!
//! - **[`HookEvent`]** — enum of 18 lifecycle events (PreToolUse, PostToolUse, etc.)
//! - **[`HookInput`]** — typed payload for each event, carrying context like session ID,
//!   tool name, and tool input/output
//! - **[`HookOutput`]** — return value from hooks, either async (fire-and-forget) or
//!   sync (with decisions like approve/block)
//! - **[`HookCallback`]** — async function signature for hook implementations
//! - **[`HookCallbackMatcher`]** — groups callbacks with an optional regex filter
//! - **[`HookRegistry`]** — manages hooks by event type and runs them
//!
//! ## Quick Start
//!
//! ```rust
//! use starpod_hooks::{HookRegistry, HookEvent, HookCallbackMatcher, hook_fn, HookOutput};
//!
//! let mut registry = HookRegistry::new();
//!
//! // Register a hook that fires after any Bash tool use
//! registry.register(HookEvent::PostToolUse, vec![
//!     HookCallbackMatcher::new(vec![
//!         hook_fn(|input, _id, _cancel| async move {
//!             println!("Tool used: {}", input.tool_name().unwrap_or("unknown"));
//!             Ok(HookOutput::default())
//!         }),
//!     ])
//!     .with_matcher("Bash")
//!     .with_timeout(30),
//! ]);
//! ```
//!
//! ## Hook Events
//!
//! | Event | When it fires | Can block? |
//! |-------|--------------|------------|
//! | `PreToolUse` | Before tool execution | Yes — can modify input or deny |
//! | `PostToolUse` | After successful tool execution | No |
//! | `PostToolUseFailure` | After failed tool execution | No |
//! | `UserPromptSubmit` | When user sends a message | Yes |
//! | `SessionStart` | Session begins | No |
//! | `SessionEnd` | Session ends | No |
//! | `Stop` | Agent stopping | No |
//! | `Notification` | System notification | No |
//! | `SubagentStart` | Subagent launching | No |
//! | `SubagentStop` | Subagent finished | No |
//! | `PreCompact` | Before conversation compaction | No |
//! | `PermissionRequest` | Permission decision needed | Yes |
//! | `Setup` | Initial/maintenance setup | No |
//! | `TeammateIdle` | Teammate idle | No |
//! | `TaskCompleted` | Task finished | No |
//! | `ConfigChange` | Configuration changed | No |
//! | `WorktreeCreate` | Git worktree created | No |
//! | `WorktreeRemove` | Git worktree removed | No |

pub mod callback;
pub mod error;
pub mod event;
pub mod input;
pub mod output;
pub mod permissions;
pub mod runner;

// Re-export main public API
pub use callback::{hook_fn, HookCallback, HookCallbackMatcher};
pub use error::HookError;
pub use event::HookEvent;
pub use input::{
    BaseHookInput, CompactTriggerType, ConfigChangeSource, HookInput, SessionStartSource,
    SetupTrigger,
};
pub use output::{
    AsyncHookOutput, HookDecision, HookOutput, HookSpecificOutput, PermissionRequestDecision,
    SyncHookOutput,
};
pub use permissions::{PermissionDecision, PermissionLevel, PermissionUpdate};
pub use runner::HookRegistry;
