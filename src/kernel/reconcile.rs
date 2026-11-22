//! Declarative YAML profile, bundle, and patch resolution for Cordis reconciliation.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::kernel::error::KernelError;
use crate::kernel::fiber::Plugin;

/// Single plugin declaration inside a Cordis profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileEntry {
    /// Unique fiber identifier within the kernel.
    pub id: String,
    /// Plugin factory key registered in `PluginFactoryRegistry`.
    pub plugin: String,
    /// Whether this fiber should be active (`true`) or unmounted (`false`).
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Arbitrary JSON-compatible configuration passed to `Plugin::mount`.
    #[serde(default = "default_empty_object")]
    pub config: Value,
}

fn default_enabled() -> bool {
    true
}

fn default_empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}

/// Top-level `cordis.yml` document containing named profiles (`serve`, `mcp-stdio`, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CordisConfig {
    /// Map of profile name to ordered plugin declarations.
    #[serde(default)]
    pub profiles: HashMap<String, Vec<ProfileEntry>>,
}

/// Individual override rule in `cordis.patch.yml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatchEntry {
    /// Target fiber ID to modify (or append if `plugin` is set and ID is new).
    pub id: String,
    /// Optional plugin factory override or new plugin factory when adding an entry.
    #[serde(default)]
    pub plugin: Option<String>,
    /// Optional override for whether the plugin is enabled.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Optional partial configuration deep-merged onto the entry's config object.
    #[serde(default)]
    pub config: Option<Value>,
}

/// Declarative `cordis.patch.yml` document.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CordisPatch {
    /// Ordered patch entries merged onto the selected profile.
    #[serde(default)]
    pub patches: Vec<PatchEntry>,
}

/// Factory function instantiating a `Plugin` from a `PluginFactoryRegistry`.
pub type PluginFactory = Arc<dyn Fn() -> Arc<dyn Plugin> + Send + Sync>;

/// Registry mapping YAML `plugin:` identifiers to compiled-in plugin constructors.
#[derive(Default)]
pub struct PluginFactoryRegistry {
    factories: HashMap<String, PluginFactory>,
}

impl std::fmt::Debug for PluginFactoryRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut keys: Vec<&str> = self.factories.keys().map(String::as_str).collect();
        keys.sort_unstable();
        f.debug_struct("PluginFactoryRegistry")
            .field("factories", &keys)
            .finish()
    }
}

impl PluginFactoryRegistry {
    /// Creates an empty plugin factory registry.
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Registers a compiled-in `Plugin` type under `name`.
    pub fn register<F>(&mut self, name: impl Into<String>, ctor: F)
    where
        F: Fn() -> Arc<dyn Plugin> + Send + Sync + 'static,
    {
        self.factories.insert(name.into(), Arc::new(ctor));
    }

    /// Instantiates a plugin by factory name.
    ///
    /// # Errors
    /// Returns `KernelError::UnknownPlugin` if `name` is not registered.
    pub fn create(&self, name: &str) -> Result<Arc<dyn Plugin>, KernelError> {
        let ctor = self
            .factories
            .get(name)
            .ok_or_else(|| KernelError::UnknownPlugin(name.to_string()))?;
        Ok(ctor())
    }
}

/// Deep-merges `overlay` onto `base` for JSON objects; replaces arrays and scalars.
pub fn deep_merge_json(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (k, v) in overlay_map {
                match base_map.get_mut(k) {
                    Some(existing) => deep_merge_json(existing, v),
                    None => {
                        base_map.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (base_slot, overlay_val) => {
            *base_slot = overlay_val.clone();
        }
    }
}

/// Resolves a named profile from `config` and applies `patch` overrides non-destructively.
///
/// # Errors
/// Returns `KernelError::UnknownProfile` if `profile` does not exist, or
/// `KernelError::ConfigError` if a new patch entry lacks a `plugin` factory name.
pub fn resolve_profile(
    config: &CordisConfig,
    patch: Option<&CordisPatch>,
    profile: &str,
) -> Result<Vec<ProfileEntry>, KernelError> {
    let base_entries = config
        .profiles
        .get(profile)
        .ok_or_else(|| KernelError::UnknownProfile(profile.to_string()))?;

    let mut resolved = base_entries.clone();

    if let Some(patch_doc) = patch {
        for p in &patch_doc.patches {
            if let Some(existing) = resolved.iter_mut().find(|e| e.id == p.id) {
                if let Some(plugin_name) = &p.plugin {
                    existing.plugin = plugin_name.clone();
                }
                if let Some(enabled) = p.enabled {
                    existing.enabled = enabled;
                }
                if let Some(cfg_patch) = &p.config {
                    deep_merge_json(&mut existing.config, cfg_patch);
                }
            } else {
                let plugin_name = p.plugin.clone().ok_or_else(|| {
                    KernelError::ConfigError(format!(
                        "patch entry '{}' adds a new fiber without specifying 'plugin'",
                        p.id
                    ))
                })?;
                resolved.push(ProfileEntry {
                    id: p.id.clone(),
                    plugin: plugin_name,
                    enabled: p.enabled.unwrap_or(true),
                    config: p.config.clone().unwrap_or_else(default_empty_object),
                });
            }
        }
    }

    Ok(resolved)
}
