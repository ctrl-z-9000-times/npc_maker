//! Environment Interface, for making and using environments.
//!
//! Each environment runs in its own computer process and uses stdin and stdout
//! to communicate with the main program and the evolutionary algorithm which
//! it contains. Environments should use stderr to report any diagnostic or
//! unformatted messages(see [eprintln!()]).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Static description of an environment and its interfaces.  
/// Each environment specification file contains one of these.  
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EnvironmentSpec {
    /// Filesystem path to the environment’s static specification (this file).
    #[serde(skip)]
    pub spec: PathBuf,

    /// Name of the environment, should be universally unique.
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

    pub fn get_args(&self, graphical: bool, settings: &HashMap<String, String>) -> Vec<String> {
        // Setup the program's command line invocation and marshal its arguments.
        let mut args: Vec<String> = vec![
            self.path.to_str().unwrap().to_string(),
            self.spec.to_str().unwrap().to_string(),
            (if graphical { "graphical" } else { "headless" }).to_string(),
        ];
        args.reserve(2 * self.settings.len());
        for parameter in &self.settings {
            let name = parameter.name();
            let value = match settings.get(name) {
                Some(r#override) => r#override.clone(),
                None => parameter.default(),
            };
            args.push(name.to_string());
            args.push(value);
        }
        args
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

/// Display mode for environments.
#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Disable graphical output.
    ///
    /// The environment should run as quickly and quietly as possible.
    Headless,

    /// Display graphical output to the user.
    ///
    /// This mode is for demonstrations and so the environment should run in as
    /// close to real time as possible and with full user interactivity enabled.
    ///
    /// The environment may also print diagnostic information to stderr.
    #[default]
    Graphical,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Graphical => write!(f, "graphical"),
            Mode::Headless => write!(f, "headless"),
        }
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_ascii_lowercase();
        match s.as_str() {
            "graphical" => Ok(Mode::Graphical),
            "headless" => Ok(Mode::Headless),
            _ => Err(format!("expected either \"graphical\" or \"headless\", got \"{s}\"")),
        }
    }
}

impl From<bool> for Mode {
    /// Converts false to headless,  
    /// Converts true to graphical.  
    fn from(mode: bool) -> Self {
        if mode {
            Mode::Graphical
        } else {
            Mode::Headless
        }
    }
}

/// Read the command line arguments for an environment program.
///
/// Returns a tuple of (environment-specification, graphics-mode, settings-dict)
pub fn get_args() -> (EnvironmentSpec, Mode, HashMap<String, String>) {
    // Read the command line arguments.
    let mut arg_iter = std::env::args();
    let _program = arg_iter.next();
    let spec_file = arg_iter.next();
    let mode = arg_iter.next();
    let mut settings: Vec<String> = arg_iter.collect();
    // Read the environment specification file.
    let Some(spec_file) = spec_file else {
        panic!("Argument Error: missing environment specification")
    };
    let spec_file = Path::new(&spec_file)
        .canonicalize()
        .unwrap_or_else(|err| panic!("File Error: {err}: {spec_file:?}"));
    let spec_data =
        std::fs::read_to_string(&spec_file).unwrap_or_else(|err| panic!("File Error: {err}: {spec_file:?}"));
    let mut env_spec: EnvironmentSpec =
        serde_json::from_str(&spec_data).unwrap_or_else(|err| panic!("JSON Decode Error: {err}: {spec_file:?}"));
    env_spec.spec = spec_file;
    // Read the graphics mode.
    let mode = if let Some(mode) = mode {
        mode.parse().unwrap_or_else(|err| panic!("Argument Error: {err}"))
    } else {
        Mode::default()
    };
    // Assemble the settings dictionary.
    let mut defaults: HashMap<String, _> = env_spec
        .settings
        .iter()
        .map(|item| (item.name().to_string(), item.default()))
        .collect();
    let mut settings = settings.chunks_exact_mut(2);
    for chunk in &mut settings {
        let item = std::mem::take(&mut chunk[0]);
        let value = std::mem::take(&mut chunk[1]);
        if !defaults.contains_key(&item) {
            panic!("Argument Error: unexpected parameter \"{item}\"")
        }
        defaults.insert(item, value);
    }
    if !settings.into_remainder().is_empty() {
        panic!("Argument Error: odd number of settings, expected key-value pairs");
    }
    //
    (env_spec, mode, defaults)
}

/// Read the next individual from the evolution program, blocking.
///
/// New individual must be requested before calling this with the [spawn()] and
/// [mate()] functions.
pub fn input() -> Result<(Individual, Box<[u8]>), io::Error> {
    io::stdout().flush().unwrap();

    let mut line = String::new();
    let stdin = &mut io::stdin().lock();
    stdin.read_line(&mut line)?;

    let metadata: Individual = serde_json::from_str(&line)?;

    let binary = crate::read_bytes(stdin, metadata.genome)?;

    Ok((metadata, binary))
}

/// Metadata for an individual.
///
/// The evolution process sends an Individual encoded in UTF-8 JSON on a single
/// line, immediately followed by the individual's genome as a binary array.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Individual {
    pub name: String,

    #[serde(default)]
    pub population: String,

    #[serde(default)]
    pub parents: Vec<String>,

    /// The command line invocation for the controller program
    pub controller: Vec<String>,

    /// Number of bytes in the genome
    pub genome: usize,

    /// Non-standard fields
    #[serde(flatten)]
    pub other: HashMap<String, String>,
}

/// Structure of all messages sent from the environment instances to the evolution process.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum EvolutionMessage {
    /// Request a new individual from the evolutionary algorithm.
    Spawn {
        #[serde(rename = "Spawn", default)]
        population: String,
    },

    /// Request to mate two individuals.
    /// Both individuals must still be alive and in the environment.
    Mate {
        #[serde(rename = "Mate")]
        parents: [String; 2],
    },

    /// Report the score or reproductive fitness of an individual.
    Score {
        #[serde(rename = "Score")]
        score: String,
        name: String,
    },

    /// Associate some extra information with an individual.
    Telemetry {
        #[serde(rename = "Telemetry")]
        info: HashMap<String, String>,
        name: String,
    },

    /// Report the death of an individual.
    Death {
        #[serde(rename = "Death")]
        name: String,
    },
}

/// Request a new individual from the evolutionary algorithm.
///
/// Argument population is optional if the environment contains exactly one population.
pub fn spawn(population: Option<&str>) {
    println!(r#"{{"Spawn":"{}"}}"#, population.unwrap_or(""));
}

/// Request to mate two specific individuals together to produce a child individual.
pub fn mate(parent1: &str, parent2: &str) {
    println!(r#"{{"Mate":["{parent1}","{parent2}"]}}"#);
}

/// Report an individual's score or reproductive fitness to the evolutionary algorithm.
///
/// This should be called *before* calling [death] on the individual.
///
/// Argument individual is optional if the environment contains exactly one individual.
pub fn score(individual: Option<&str>, value: &str) {
    println!(r#"{{"Score":"{value}","name":"{}"}}"#, individual.unwrap_or(""));
}

/// Report extra information about an individual.
///
/// Argument info is a mapping of string key-value pairs.
///
/// Argument individual is optional if the environment contains exactly one individual.
pub fn telemetry(individual: Option<&str>, info: &HashMap<String, String>) {
    let mut json = String::new();
    for (key, value) in info {
        json.push('"');
        json.push_str(key);
        json.push('"');
        json.push(':');
        json.push('"');
        json.push_str(value);
        json.push('"');
        json.push(',');
    }
    json.pop(); // Remove trailing comma.
    println!(r#"{{"Telemetry":{{{json}}},"name":"{}"}}"#, individual.unwrap_or(""));
}

/// Notify the evolutionary algorithm that the given individual has died.
///
/// The individual's score or reproductive fitness should be reported
/// using the [score()] function *before* calling this method.
///
/// Argument individual is optional if the environment contains exactly one individual.
pub fn death(individual: Option<&str>) {
    println!(r#"{{"Death":"{}"}}"#, individual.unwrap_or(""));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse all of the env-spec's in the examples directory.
    #[test]
    fn test_example_environment_specifications() {
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

    /// Check that the messages being sent to the environment are exactly as expected.
    #[test]
    fn send_string() {
        assert_eq!(
            serde_json::to_string(&Individual {
                name: "1234".to_string(),
                population: "pop1".to_string(),
                parents: vec!["1020".to_string(), "1077".to_string()],
                controller: vec!["/usr/bin/q".to_string()],
                genome: 456789,
                other: Default::default(),
            })
            .unwrap(),
            r#"{"name":"1234","population":"pop1","parents":["1020","1077"],"controller":["/usr/bin/q"],"genome":456789}"#
        );
    }

    /// Check that the messages received from the environment are exactly as expected.
    #[test]
    fn recv_string() {
        assert_eq!(
            serde_json::to_string(&EvolutionMessage::Spawn {
                population: String::new()
            })
            .unwrap(),
            r#"{"Spawn":""}"#
        );
        assert_eq!(
            serde_json::to_string(&EvolutionMessage::Spawn {
                population: "pop1".to_string()
            })
            .unwrap(),
            r#"{"Spawn":"pop1"}"#
        );
        assert_eq!(
            serde_json::to_string(&EvolutionMessage::Mate {
                parents: ["parent1".to_string(), "parent2".to_string()]
            })
            .unwrap(),
            r#"{"Mate":["parent1","parent2"]}"#
        );
        assert_eq!(
            serde_json::to_string(&EvolutionMessage::Score {
                name: "xyz".to_string(),
                score: "-3.7".to_string(),
            })
            .unwrap(),
            r#"{"Score":"-3.7","name":"xyz"}"#
        );
        assert_eq!(
            serde_json::to_string(&EvolutionMessage::Telemetry {
                name: "abcd".to_string(),
                info: HashMap::new()
            })
            .unwrap(),
            r#"{"Telemetry":{},"name":"abcd"}"#
        );
        assert_eq!(
            serde_json::to_string(&EvolutionMessage::Death { name: String::new() }).unwrap(),
            r#"{"Death":""}"#
        );
        assert_eq!(
            serde_json::to_string(&EvolutionMessage::Death {
                name: "abc".to_string()
            })
            .unwrap(),
            r#"{"Death":"abc"}"#
        );
    }
}
