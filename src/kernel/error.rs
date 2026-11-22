//! Error types emitted by the zero-domain Cordis plugin kernel.

use std::fmt;

/// Errors produced by Cordis kernel lifecycle, service injection, or profile reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// Attempted to mount a fiber whose identifier is already registered.
    DuplicateFiber(String),
    /// Referenced a fiber identifier that is not present in the kernel.
    FiberNotFound(String),
    /// Requested a required service (`TypeId`) that is not currently provided.
    ServiceNotRegistered(&'static str),
    /// Detected a dependency cycle among plugin `provides` / `requires` declarations.
    CycleDetected(String),
    /// A plugin failed during its asynchronous `mount` phase.
    MountFailed {
        /// Identifier of the fiber that failed to mount.
        fiber_id: String,
        /// Human-readable error message.
        reason: String,
    },
    /// Invalid or malformed YAML configuration or patch specification.
    ConfigError(String),
    /// Requested a profile name that does not exist in `cordis.yml`.
    UnknownProfile(String),
    /// Referenced a plugin factory name that is not registered.
    UnknownPlugin(String),
    /// Internal runtime or synchronization failure.
    Internal(String),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateFiber(id) => write!(f, "duplicate fiber id: {id}"),
            Self::FiberNotFound(id) => write!(f, "fiber not found: {id}"),
            Self::ServiceNotRegistered(name) => {
                write!(f, "required service not registered: {name}")
            }
            Self::CycleDetected(desc) => write!(f, "plugin dependency cycle detected: {desc}"),
            Self::MountFailed { fiber_id, reason } => {
                write!(f, "failed to mount fiber '{fiber_id}': {reason}")
            }
            Self::ConfigError(msg) => write!(f, "configuration error: {msg}"),
            Self::UnknownProfile(name) => write!(f, "unknown profile: {name}"),
            Self::UnknownPlugin(name) => write!(f, "unknown plugin factory: {name}"),
            Self::Internal(msg) => write!(f, "kernel internal error: {msg}"),
        }
    }
}

impl std::error::Error for KernelError {}
