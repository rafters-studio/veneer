use crate::error::RegistryError;
use crate::model::{NamespaceTokenFile, RaftersConfig, RegistryIndex, RegistryItem};

/// Trait for reading rafters registry data from any source.
pub trait RegistryReader {
    /// Load all namespace token files from `.rafters/tokens/`.
    fn load_namespaces(&self) -> Result<Vec<NamespaceTokenFile>, RegistryError>;

    /// Load a single namespace token file by name (e.g. "color").
    fn load_namespace(&self, name: &str) -> Result<NamespaceTokenFile, RegistryError>;

    /// Load the registry index from `.rafters/registry/index.json`.
    fn load_index(&self) -> Result<RegistryIndex, RegistryError>;

    /// Load a component by name from `.rafters/registry/components/{name}.json`.
    fn load_component(&self, name: &str) -> Result<RegistryItem, RegistryError>;

    /// Load a primitive by name from `.rafters/registry/primitives/{name}.json`.
    fn load_primitive(&self, name: &str) -> Result<RegistryItem, RegistryError>;

    /// Load a composite by name from `.rafters/registry/composites/{name}.json`.
    fn load_composite(&self, name: &str) -> Result<RegistryItem, RegistryError>;

    /// Load the rafters config from `.rafters/config.rafters.json`.
    fn load_config(&self) -> Result<RaftersConfig, RegistryError>;
}
