//! Structure of environment specification files.

use crate::serde_utils::{deserialize_positive, multiline_string, required_string, JsonIoError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Static description of an environment and its interfaces.  
/// Each environment specification file contains one of these.  
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EnvironmentSpec {
    /// Filesystem path to the environment’s static specification (this file).
    #[serde(skip)]
    pub spec: PathBuf,

    /// Name of the environment, must be unique.
    #[serde(deserialize_with = "required_string")]
    pub name: String,

    /// Filesystem path of the environment’s executable program, relative to this file.
    pub path: PathBuf,

    /// Specification for each population.
    #[serde(default)]
    pub populations: Vec<PopulationSpec>,

    /// Settings menu items for the user to customize the environment.
    #[serde(default)]
    pub settings: Vec<SettingsSpec>,

    /// User facing documentation message.
    #[serde(default, deserialize_with = "multiline_string")]
    pub description: String,

    /// Request environmental control over the mating process.
    #[serde(default)]
    pub mating: bool,

    /// Restrict this environment to a single instance on each computer.
    #[serde(default)]
    pub global: bool,

    /// Estimated number of concurrent threads of computation.
    #[serde(default = "default_one")]
    pub threads: u32,

    /// Estimated peak memory usage, measured in gigabytes.
    #[serde(default, deserialize_with = "deserialize_positive")]
    pub memory: f64,
}

impl EnvironmentSpec {
    /// Load an environment specification from a JSON file.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, JsonIoError> {
        let path = path.as_ref(); // Convert into a proper &Path.
        let spec = std::fs::read_to_string(path)?;
        // .unwrap_or_else(|err| panic!("error reading file {path:?} {err}"));
        let mut this: EnvironmentSpec = serde_json::from_str(&spec)?;
        // .unwrap_or_else(|err| panic!("error parsing JSON file {path:?} {err}",));
        this.spec = path.into();
        Ok(this)
    }

    /// Sanity checks on the environment specification file, panics on failure.
    pub fn validate(&self) -> Result<(), String> {
        let Self { spec, path, .. } = self;
        if spec == &PathBuf::default() {
            return Err("environment specification was not loaded from file".to_string());
        }
        // Check that the environment program exists.
        if !path.exists() {
            return Err(format!("file not found {path:?}"));
        }
        if !path.is_file() {
            return Err(format!("not a file {path:?}"));
        }
        // Check that the interface GIN's are unique.
        for pop_spec in &self.populations {
            let unique_gins: HashSet<u64> = pop_spec.interfaces.iter().map(|interface| interface.gin).collect();
            if unique_gins.len() < pop_spec.interfaces.len() {
                return Err(format!("interface has duplicate \"gin\", in file: {spec:?}"));
            }
        }
        Ok(())
    }
}

/// Description for each specific population within an environment.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PopulationSpec {
    /// Name of the population, must be unique within the environment.
    pub name: String,

    /// User facing documentation message.
    #[serde(default, deserialize_with = "multiline_string")]
    pub description: String,

    /// Genetic interface for this lifeform’s body.
    #[serde(default)]
    pub interfaces: Vec<InterfaceSpec>,
}

/// Description of the interface between a body and its genotype.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InterfaceSpec {
    /// Global Innovation Number, must be unique within the interfaces array.
    pub gin: u64,

    /// User facing name for this chromosome.
    #[serde(deserialize_with = "required_string")]
    pub name: String,

    /// List of acceptable chromosome types for this interface.  
    #[serde(default)]
    pub chromosome_types: Vec<Arc<str>>,

    /// User facing documentation message.
    #[serde(default, deserialize_with = "multiline_string")]
    pub description: String,
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

        #[serde(default, deserialize_with = "multiline_string")]
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

        #[serde(default, deserialize_with = "multiline_string")]
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

        #[serde(default, deserialize_with = "multiline_string")]
        description: String,

        /// Initial value for new environments.
        default: bool,
    },

    #[serde(alias = "enum")]
    Enumeration {
        name: String,

        #[serde(default, deserialize_with = "multiline_string")]
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

const fn default_one() -> u32 {
    1
}
