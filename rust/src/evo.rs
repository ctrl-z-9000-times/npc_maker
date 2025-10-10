use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

/// API for evolutionary algorithms.
pub trait Evolution {
    /// Get a new individual to be born into an environment.
    ///
    /// Returns an individual and a genome for the controller, which may differ
    /// from the individual's genome.
    fn spawn(&self) -> (Individual, Box<[u8]>);

    /// Notification of an individual's death.
    fn death(&self, individual: Individual);
}

fn uuid4() -> String {
    let rng = &mut rand::rng();
    let uuid = rng.random::<u128>();
    format!("{uuid}")
}

#[derive(Serialize, Deserialize)]
pub struct Individual {
    /// Name or UUID of this individual.
    pub name: String,

    /// Number of individuals who died before this one,
    /// or None if this individual has not yet died.
    pub ascension: Option<u64>,

    /// Name of the environment that this individual lives in.
    pub environment: String,

    /// Name of this population that this individual belongs to.
    pub population: String,

    /// UUID of this individual's species.
    /// Mating may be restricted to individuals of the same species.
    pub species: String,

    /// Command line invocation of the controller program.
    pub controller: Vec<String>,

    /// Genetic parameters for this AI agent.
    #[serde(skip)]
    pub genome: Box<[u8]>,

    /// The environmental info dictionary. The environment updates this information.
    pub telemetry: HashMap<String, String>,

    /// The epigenetic info dictionary. The controller updates this information.
    pub epigenome: HashMap<String, String>,

    /// Reproductive fitness of this individual, as assessed by the environment.
    pub score: String,

    /// Number of cohorts that passed before this individual was born.
    pub generation: u64,

    /// The names of this individual's parents.
    pub parents: Vec<String>,

    /// The names of this individual's children.
    pub children: Vec<String>,

    /// Time of birth, as a UTC timestamp, or an empty string if this individual
    /// has not yet been born.
    pub birth_date: String,

    /// Time of death, as a UTC timestamp, or an empty string if this individual
    /// has not yet died.
    pub death_date: String,

    /// Unrecognized fields in the JSON object.
    pub extra: HashMap<String, serde_json::Value>,

    /// Get the file path this individual was loaded from or saved to, or None
    /// if this individual has not touched the file system.
    #[serde(skip)]
    pub path: Option<PathBuf>,
}

impl Individual {
    /// Create a new individual.
    pub fn new(environment: String, population: String, controller: Vec<String>, genome: Box<[u8]>) -> Self {
        Self {
            name: uuid4(),
            ascension: None,
            environment,
            population,
            species: uuid4(),
            controller,
            genome,
            telemetry: HashMap::new(),
            epigenome: HashMap::new(),
            score: String::new(),
            generation: 0,
            parents: Vec::new(),
            children: Vec::new(),
            birth_date: String::new(),
            death_date: String::new(),
            extra: HashMap::new(),
            path: None,
        }
    }

    /// Asexually reproduce an individual.
    pub fn clone(&mut self, clone_genome: impl FnOnce(&[u8]) -> Box<[u8]>) -> Self {
        let individual = Self {
            name: uuid4(),
            ascension: None,
            environment: self.environment.clone(),
            population: self.population.clone(),
            species: self.species.clone(),
            controller: self.controller.clone(),
            genome: clone_genome(&self.genome),
            telemetry: HashMap::new(),
            epigenome: HashMap::new(),
            score: String::new(),
            generation: self.generation + 1,
            parents: vec![self.name.clone()],
            children: vec![],
            birth_date: String::new(),
            death_date: String::new(),
            extra: HashMap::new(),
            path: None,
        };
        self.children.push(individual.name.clone());
        individual
    }

    /// Sexually reproduce two individuals.
    pub fn mate(&mut self, other: &mut Self, mate_genomes: impl FnOnce(&[u8], &[u8]) -> Box<[u8]>) -> Self {
        let individual = Self {
            name: uuid4(),
            ascension: None,
            environment: self.environment.clone(),
            population: self.population.clone(),
            species: self.species.clone(),
            controller: self.controller.clone(),
            genome: mate_genomes(&self.genome, &other.genome),
            telemetry: HashMap::new(),
            epigenome: HashMap::new(),
            score: String::new(),
            generation: self.generation.max(other.generation) + 1,
            parents: vec![self.name.clone(), other.name.clone()],
            children: vec![],
            birth_date: String::new(),
            death_date: String::new(),
            extra: HashMap::new(),
            path: None,
        };
        self.children.push(individual.name.clone());
        other.children.push(individual.name.clone());
        individual
    }

    /// Save an individual to a file.
    ///
    /// Argument path is the directory to save in.
    ///
    /// The filename will be the individual's name with a ".indiv" file extension.
    ///
    /// Returns the file path of the saved individual.
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<PathBuf, std::io::Error> {
        let path: &Path = path.as_ref();
        if !path.exists() {
            std::fs::create_dir(path)?;
        }
        assert!(path.is_dir());
        let path = path.join(format!("{}.indiv", self.name));
        let mut file = File::create(&path)?;
        serde_json::to_writer(&file, self).unwrap();
        file.write_all(b"\0")?;
        file.write_all(&self.genome)?;
        file.sync_all()?;
        Ok(path)
    }

    /// Load a previously saved individual.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let file = std::fs::read(path.as_ref())?;
        let sentinel = file.iter().position(|byte| *byte == b'\0').unwrap_or(file.len());
        let mut individual: Individual = serde_json::from_slice(&file[..sentinel])?;
        individual.genome = file[sentinel + 1..].into();
        Ok(individual)
    }
}

impl PartialEq for Individual {
    /// Check if two individuals have the same name.
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
