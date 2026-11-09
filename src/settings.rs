//! Settings for load and export (contracts/library-api.md).

/// What a load does with a tag on a tagged string (ADR-006 section 2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TagPolicy {
    /// Drop the tag and report it as a warning.
    #[default]
    Warn,
    /// Report every tagged string as an error.
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadSettings {
    /// Property whose value is a node's identity (ADR-001 section 2.10).
    pub node_id_property: Option<String>,
    /// Property whose value is a relationship's identity.
    pub relationship_id_property: Option<String>,
    pub tags: TagPolicy,
    /// Cap on the errors and positions a report lists (ADR-007 section 2.2).
    pub max_diagnostics: usize,
    /// The file's name, used only to check its kind (ADR-003 section 2.1).
    pub file_name: Option<String>,
}

impl Default for LoadSettings {
    fn default() -> Self {
        LoadSettings {
            node_id_property: None,
            relationship_id_property: None,
            tags: TagPolicy::Warn,
            max_diagnostics: 100,
            file_name: None,
        }
    }
}

/// How an export chooses element identifiers (ADR-002 section 2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IdentityMode {
    #[default]
    Auto,
    Generate,
    Property,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ExportSettings {
    pub identity: IdentityMode,
    pub node_id_property: Option<String>,
    pub relationship_id_property: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_defaults() {
        let s = LoadSettings::default();
        assert_eq!(s.node_id_property, None);
        assert_eq!(s.relationship_id_property, None);
        assert_eq!(s.tags, TagPolicy::Warn);
        assert_eq!(s.max_diagnostics, 100);
        assert_eq!(s.file_name, None);
    }

    #[test]
    fn export_defaults() {
        let s = ExportSettings::default();
        assert_eq!(s.identity, IdentityMode::Auto);
        assert_eq!(s.node_id_property, None);
        assert_eq!(s.relationship_id_property, None);
    }
}
