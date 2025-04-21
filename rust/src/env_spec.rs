//! Structure of environment specification files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Static description of an environment and its interfaces.  
/// Each environment specification file contains one of these.  
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EnvironmentSpec {
    /// Filesystem path to the environment’s static specification (this file).
    #[serde(skip)]
    pub spec: PathBuf,

    /// Name of the environment, should be globally unique.
    pub name: String,

    /// Filesystem path of the environment’s executable program, relative to this file.
    pub path: PathBuf,

    /// User facing documentation message.
    #[serde(default)]
    pub description: String,

    /// Specification for each population.
    #[serde(default)]
    pub populations: Vec<PopulationSpec>,

    /// Settings menu items for the user to customize the environment.
    #[serde(default)]
    pub settings: Vec<SettingsSpec>,

    /// Environments may include extra information.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl EnvironmentSpec {
    /// Load an environment specification from a JSON file.
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref(); // Convert into a proper &Path.
        let spec = std::fs::read_to_string(path).unwrap_or_else(|err| panic!("error reading file {path:?} {err}"));
        let mut this: EnvironmentSpec =
            serde_json::from_str(&spec).unwrap_or_else(|err| panic!("error parsing JSON file {path:?} {err}",));
        this.spec = path.into();
        this.normalize_path();
        this
    }

    fn normalize_path(&mut self) {
        if self.path.is_relative() {
            self.path = self.spec.parent().unwrap().join(&self.path)
        }
        // assert!(self.path.exists(), "file not found {:?}", self.path);
        // assert!(self.path.is_file(), "not a file {:?}", self.path);
    }
}

/// Description for each specific population within an environment.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PopulationSpec {
    /// Name of the population, must be unique within the environment.
    pub name: String,

    /// User facing documentation message.
    #[serde(default)]
    pub description: String,

    /// Genetic interface for this lifeform’s body.
    #[serde(default)]
    pub interfaces: Vec<InterfaceSpec>,

    /// Populations may include extra information.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Description of the interface between a body and its controller.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InterfaceSpec {
    /// Global Innovation Number.
    pub gin: u64,

    /// User facing name for this interface.
    pub name: String,

    /// User facing documentation message.
    #[serde(default)]
    pub description: String,

    /// Interfaces may include extra information.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Description of an environmental parameter.
///
/// These are presented in the graphical user interface in the settings menu for
/// this environment.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
pub enum SettingsSpec {
    #[serde(alias = "float")]
    Real {
        name: String,

        #[serde(default)]
        description: String,

        /// Lower bound on the range of allowable values, inclusive.
        minimum: f64,

        /// Upper bound on the range of allowable values, inclusive.
        maximum: f64,

        /// Initial value for new environments.
        default: f64,
    },

    #[serde(alias = "int")]
    Integer {
        name: String,

        #[serde(default)]
        description: String,

        /// Lower bound on the range of allowable values, inclusive.
        minimum: i64,

        /// Upper bound on the range of allowable values, inclusive.
        maximum: i64,

        /// Initial value for new environments.
        default: i64,
    },

    #[serde(alias = "bool")]
    Boolean {
        name: String,

        #[serde(default)]
        description: String,

        /// Initial value for new environments.
        default: bool,
    },

    #[serde(alias = "enum")]
    Enumeration {
        name: String,

        #[serde(default)]
        description: String,

        /// Names of all of the variants of the enumeration.
        values: Vec<String>,

        /// Initial value for new environments.
        default: String,
    },
}

impl SettingsSpec {
    /// Name of this settings menu item, must be unique within the environment.
    pub fn name(&self) -> &str {
        match self {
            Self::Real { name, .. } => name,
            Self::Integer { name, .. } => name,
            Self::Boolean { name, .. } => name,
            Self::Enumeration { name, .. } => name,
        }
    }

    /// User facing documentation message.
    pub fn description(&self) -> &str {
        match self {
            Self::Real { description, .. } => description,
            Self::Integer { description, .. } => description,
            Self::Boolean { description, .. } => description,
            Self::Enumeration { description, .. } => description,
        }
    }

    /// Data type.
    pub fn r#type(&self) -> &str {
        match self {
            Self::Real { .. } => "Real",
            Self::Integer { .. } => "Integer",
            Self::Boolean { .. } => "Boolean",
            Self::Enumeration { .. } => "Enumeration",
        }
    }

    pub fn default(&self) -> String {
        match self {
            Self::Real { default, .. } => default.to_string(),
            Self::Integer { default, .. } => default.to_string(),
            Self::Boolean { default, .. } => default.to_string(),
            Self::Enumeration { default, .. } => default.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse all of the env-spec's in the examples directory.
    #[test]
    fn test_examples() {
        let mut examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        examples_dir.pop();
        examples_dir.push("examples");
        for entry in examples_dir.read_dir().unwrap() {
            let Ok(entry) = entry else { continue };
            if entry.path().is_dir() {
                for file in entry.path().read_dir().unwrap() {
                    let Ok(file) = file else { continue };
                    if let Some(ext) = file.path().extension() {
                        if ext == "env" {
                            println!("EnvironmentSpec::new(\"{}\")", file.path().display());
                            EnvironmentSpec::new(file.path());
                        }
                    }
                }
            }
        }
    }
}
