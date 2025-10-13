use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

// TODO: Consider lazy loading the genome from file when its requested.

fn uuid4() -> String {
    let rng = &mut rand::rng();
    let uuid = rng.random::<u128>();
    format!("{uuid:032X}")
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
    pub other: HashMap<String, serde_json::Value>,

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
            other: HashMap::new(),
            path: None,
        }
    }

    /// Asexually reproduce an individual.
    pub fn asex(&mut self, clone_genome: impl FnOnce(&[u8]) -> Box<[u8]>) -> Self {
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
            other: HashMap::new(),
            path: None,
        };
        self.children.push(individual.name.clone());
        individual
    }

    /// Sexually reproduce two individuals.
    pub fn sex(&mut self, other: &mut Self, mate_genomes: impl FnOnce(&[u8], &[u8]) -> Box<[u8]>) -> Self {
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
            other: HashMap::new(),
            path: None,
        };
        self.children.push(individual.name.clone());
        other.children.push(individual.name.clone());
        individual
    }

    /// Save an individual to a file.
    ///
    /// Argument path is the directory to save in. Optional, use empty string
    /// for temporary file. The filename will be the individual's name with the
    /// ".indiv" file extension.
    ///
    /// Returns the file path of the saved individual.
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<PathBuf, std::io::Error> {
        let mut path: PathBuf = path.as_ref().into();
        // Fill in default path.
        if path.to_str() == Some("") {
            path = std::env::temp_dir();
        }
        // Make the directory in case this is the first individual to be saved to it.
        if !path.exists() {
            std::fs::create_dir(&path)?;
        }
        // Make paths to temporary buffer and final file locations.
        let mut temp = std::env::temp_dir();
        temp.push(format!("{}.indiv", self.name));
        path.push(format!("{}.indiv", self.name));
        //
        let mut file = File::create(&temp)?;
        let mut buf = std::io::BufWriter::new(file);
        serde_json::to_writer(&mut buf, self).unwrap();
        buf.write_all(b"\0")?;
        buf.write_all(&self.genome)?;
        let file = buf.into_inner()?; // flush the buffer
        file.sync_all()?; // push to disk
        std::fs::rename(&temp, &path)?; // move file into place
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

fn scan_dir(path: &Path) -> Result<Vec<Individual>, std::io::Error> {
    if !path.exists() {
        std::fs::create_dir(path)?;
    }
    let mut contents = vec![];
    for file in path.read_dir()? {
        let file = file?;
        if !file.file_type()?.is_file() {
            continue;
        }
        let file_name = file.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if file_name.ends_with(".indiv") {
            let individual = Individual::load(file.path())?;
            contents.push(individual);
        }
    }
    Ok(contents)
}

pub type ScoreFn = Box<dyn Fn(&Individual) -> f64>;

/// A group of individuals that are stored together in a directory.
pub struct Population {
    path: PathBuf,

    replacement: Replacement,

    population_size: u64,

    leaderboard_size: u64,

    hall_of_fame_size: u64,

    score_fn: ScoreFn,

    ascension: u64,

    generation: u64,

    members: Vec<Individual>,

    waiting: Vec<Individual>,

    leaderboard: Vec<Individual>,

    hall_of_fame: Vec<Individual>,
}

/// Controls how a population replaces individuals once it's full.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Replacement {
    /// Do not replace individuals. The population grows without bounds.
    Unbounded,

    /// Replaces individuals at random.
    Random,

    /// Replacing the oldest member.
    Oldest,

    /// Replace the lowest scoring individual.
    Worst,

    /// Replace entire generations entirely and all at once.
    Generation,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    ascension: u64,
    generation: u64,
}

impl Population {
    pub fn new(
        path: impl AsRef<Path>,
        replacement: Replacement,
        population_size: u64,
        leaderboard_size: u64,
        hall_of_fame_size: u64,
        score_fn: Option<ScoreFn>,
    ) -> Result<Self, std::io::Error> {
        let mut path: PathBuf = path.as_ref().into();
        // Fill in default path with temp dir.
        if path.to_str() == Some("") {
            path = std::env::temp_dir();
        }
        //
        let score_fn = score_fn.unwrap_or(Box::new(|individual: &Individual| {
            individual.score.parse().unwrap_or(f64::NEG_INFINITY)
        }));
        //
        let mut this = Self {
            path,
            replacement,
            population_size,
            leaderboard_size,
            hall_of_fame_size,
            score_fn,
            ascension: 0,
            generation: 0,
            members: Default::default(),
            waiting: Default::default(),
            leaderboard: Default::default(),
            hall_of_fame: Default::default(),
        };
        this.load_metadata()?;
        this.load_members()?;
        Ok(this)
    }
    fn load_metadata(&mut self) -> Result<(), std::io::Error> {
        let path = self.get_metadata_path();
        if path.exists() {
            let metadata: Metadata = serde_json::from_slice(&std::fs::read(path)?).unwrap();
            self.ascension = metadata.ascension;
            self.generation = metadata.generation;
        }
        Ok(())
    }
    fn save_metadata(&self) -> Result<(), std::io::Error> {
        let path = self.get_metadata_path();
        let metadata = Metadata {
            ascension: self.ascension,
            generation: self.generation,
        };
        std::fs::write(path, serde_json::to_string(&metadata).unwrap())?;
        Ok(())
    }
    fn load_members(&mut self) -> Result<(), std::io::Error> {
        self.members = scan_dir(self.get_path())?;
        self.waiting = scan_dir(&self.get_waiting_path())?;
        let mut leaderboard = scan_dir(&self.get_leaderboard_path())?;
        leaderboard.sort_by(|a, b| (self.score_fn)(a).total_cmp(&(self.score_fn)(b)));
        self.leaderboard = leaderboard;
        self.hall_of_fame = scan_dir(&self.get_hall_of_fame_path())?;
        self.hall_of_fame.sort_by_key(|individual| individual.ascension);
        Ok(())
    }
    /// Get the path argument orssssssssss a temporary directory.
    pub fn get_path(&self) -> &Path {
        &self.path
    }
    fn get_metadata_path(&self) -> PathBuf {
        self.path.join("population.json")
    }
    /// Get the waiting directory. Individuals are staged here until the next
    /// generation rollover.
    fn get_waiting_path(&self) -> PathBuf {
        self.path.join("waiting")
    }
    /// Get the leaderboard path. If disabled then directory will be empty.
    fn get_leaderboard_path(&self) -> PathBuf {
        self.path.join("leaderboard")
    }
    /// Get the hall of fame path. If disabled then directory will be empty.
    fn get_hall_of_fame_path(&self) -> PathBuf {
        self.path.join("hall_of_fame")
    }
    /// Get the total number of individuals added to the population.
    pub fn get_ascension(&self) -> u64 {
        self.ascension
    }
    /// Get the number of generations that have completely passed.
    pub fn get_generation(&self) -> u64 {
        self.generation
    }
    /// Get the current members of the population.
    pub fn get_members(&self) -> &[Individual] {
        &self.members
    }
    pub fn add(&mut self, individual: &mut Individual) -> Result<(), std::io::Error> {
        individual.ascension = Some(self.ascension);
        self.ascension += 1;
        // Ignore individuals who die without a valid score.
        let score = (self.score_fn)(individual);
        if score.partial_cmp(&f64::NEG_INFINITY) != Some(std::cmp::Ordering::Greater) {
            return Ok(());
        }
        // Stage the individual in the waiting directory.
        individual.save(self.get_waiting_path())?;
        self.waiting.push(todo!());
        //
        match self.replacement {
            Replacement::Unbounded => {
                // individual.save(&self.path)?;
            }
            Replacement::Random => {}
            Replacement::Worst => {}
            Replacement::Oldest => {}
            Replacement::Generation => {}
        }

        Ok(())
    }
    fn rollover(&mut self) {
        self.rollover_leaderboard();
        self.rollover_hall_of_fame();
        self.rollover_waiting();
    }
    fn rollover_leaderboard(&mut self) {
        let leaderboard_path = self.get_leaderboard_path();
        // Sort together the existing leaderboard and the new contenders.
        let mut competition = Vec::with_capacity(self.leaderboard.len() + self.waiting.len());
        for individual in &self.leaderboard {
            competition.push((self.score(individual), true, &individual.path));
        }
        for individual in &self.waiting {
            competition.push((self.score(individual), false, &individual.path));
        }
        competition.sort_by(|a, b| a.0.total_cmp(&b.0));
        // Move new winners to the leaderboard directory.
        for (score, in_place, path) in &competition[..self.leaderboard_size as usize] {
            if !in_place {}
        }

        // Add the new generation to the leaderboard.
        // self.leaderboard.extend(&self.waiting);

        // std::fs::copy();
        // in_leaderboard = lambda path: path and path.is_relative_to(leaderboard_path)
        // self._sort_by_score(self._leaderboard_data)
        // # Discard low performing individuals.
        // while len(self._leaderboard_data) > self._leaderboard:
        //     individual = self._leaderboard_data.pop()
        //     if in_leaderboard(individual.path):
        //         individual.path.unlink()
        // # Ensure all remaining individuals are saved to the leaderboard directory.
        // for individual in self._leaderboard_data:
        //     if not in_leaderboard(individual.path):
        //         individual.path = _copy_file(individual.path, leaderboard_path)
    }
    fn rollover_hall_of_fame(&mut self) {
        //
        self.waiting
            .sort_by(|a, b| self.score(a).total_cmp(&self.score(b)).reverse());

        let winners = &mut self.waiting[..self.hall_of_fame_size as usize];
        winners.sort_unstable_by_key(|individual| individual.ascension);

        let hall_of_fame_path = self.get_hall_of_fame_path();

        for individual in winners {
            // std::fs::copy(&individual.path, hall_of_fame_path).unwrap();
            // let individual = Individual::load(path).unwrap();
            // self.hall_of_fame.push(individual);
        }
    }
    fn rollover_waiting(&mut self) {
        self.generation += 1;
        for individual in self.waiting.drain(..) {
            let path = individual.path.unwrap();
            std::fs::remove_file(path).unwrap();
        }
    }
}

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

/*
pub struct Neat {
    speciation_distance: f64,
    species_scores: Vec<f64>,
    species_members: Vec<Range<usize>>,
    parents: Vec<[usize; 2]>,
}

impl Neat {
    pub fn new(
        seed: Individual,
        population_size: u64,
        speciation_distance: f64,
        species_distribution: Box<dyn MateSelection<impl rand::Rng>>,
        mate_selection: Box<dyn MateSelection<impl rand::Rng>>,
        path: Option<impl AsRef<Path>>,
        leaderboard: u64,
        hall_of_fame: u64,
    ) -> Self {
        let path = todo!();
        Self {
            speciation_distance,
            species_scores: vec![0.0],
            species_members: vec![0..1],
            parents: vec![],
        }
    }

    fn rollover(&mut self) {
        // Sort the population by species.
        self.current.sort_unstable_by_key(|x| x.species);

        //
        let mut prev_species = None;
        let mut start = 0;
        for (index, indiv) in self.current.iter().enumerate() {
            if Some(indiv.species) != prev_species {
                self.species_members.push(start..index);
                start == index
            }
        }
        self.species_members.push(start..self.current.len());

        //
        for range in &self.species_members {
            let mut score = 0.0;
            for indiv in self.current[*range] {
                score += indiv.score.parse().unwrap();
            }
            self.species_scores.push(score / range.len() as f64);
        }
    }

    /// Refill the parents buffer.
    fn sample(&mut self) {
        let rng = &mut rand::rng();
        // Distribute the offspring to species according to their average score.
        let species = self
            .species_selection
            .select(rng, self.population_size, self.species_scores.clone());
        // Count how many offspring were allocated to each species.
        let mut histogram = vec![0; self.species_scores.len()];
        for x in species {
            histogram[x] += 1;
        }
        // Sample parents from each species.
        for (members, offspring) in self.species_members.iter().zip(&histogram) {
            let scores: Vec<f64> = members.iter().map(|x| x.score.parse().unwrap()).collect();
            for pair in self.mate_selection.pairs(offspring, scores) {
                self.parents.push(pair.map(|index| &members[index]))
            }
        }
        //
        self.parents.shuffle(rng);
    }

    pub fn birth(&mut self, _parents: &[&Individual]) -> (u128, Arc<Genome>) {
        //
        if self.parents.is_empty() {
            self.sample();
        }
        //
        let Some([mother, father]) = self.parents.pop().unwrap();
        let child = mother.mate(father).unwrap()?;

        // Determine which species the child belongs to.
        let species = if speciation_distance == 0.0 {
            self.species
        } else {
            for parent in [self, other] {
                let distance = self.genome.distance(&child);
                if distance < speciation_distance {
                    break self.species;
                }
            }
            uuid4()
        };

        //
        child
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid4_len() {
        for _ in 0..100 {
            assert_eq!(uuid4().len(), 32);
        }
    }
    #[test]
    fn uuid4_unique() {
        use std::collections::HashSet;
        let unique = 1000;
        assert_eq!((0..unique).map(|_| uuid4()).collect::<HashSet<String>>().len(), unique);
    }
    #[test]
    fn indiv_save_load() {
        let mut indiv1 = Individual::new(
            "foo".into(),
            "bar".into(),
            vec!["ctrl".into(), "prog".into()],
            Box::new(*b"beepboop"),
        );
        indiv1.other.insert("X".into(), "Y".into());
        let path = indiv1.save(std::env::temp_dir()).unwrap();
        let indiv2 = Individual::load(path).unwrap();
        assert_eq!(indiv1, indiv2);
    }
}
