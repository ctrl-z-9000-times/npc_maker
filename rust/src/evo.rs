//! Evolutionary algorithms and supporting tools.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Generate a universally unique name. This will never return the same name twice.
fn uuid4() -> String {
    let rng = &mut rand::rng();
    let uuid = rng.random::<u128>();
    format!("{uuid:032X}")
}

/// Callback for asexually reproducing a genome.
///
/// Returns a pair of (genome, phenome)
pub type CloneGenome = dyn Fn(&[u8]) -> (Box<[u8]>, Box<[u8]>);

/// Callback for sexually reproducing two genomes.
///
/// Returns a pair of (genome, phenome)
pub type MateGenomes = dyn Fn(&[u8], &[u8]) -> (Box<[u8]>, Box<[u8]>);

/// Individuals may have custom scores functions with this type signature.
///
/// By default the npc_maker will parse the individual's score into a single
/// floating point number.
pub type ScoreFn = dyn Fn(&Individual) -> f64;

const DEFAULT_SCORE: f64 = f64::NEG_INFINITY;

fn call_score_fn(score_fn: Option<&ScoreFn>, individual: &Individual) -> f64 {
    if let Some(score_fn) = score_fn {
        score_fn(individual)
    } else if let Some(score) = &individual.score {
        score.parse().unwrap_or(DEFAULT_SCORE)
    } else {
        DEFAULT_SCORE
    }
}

fn compare_scores(score_fn: Option<&ScoreFn>) -> impl Fn(&Individual, &Individual) -> std::cmp::Ordering {
    move |a, b| {
        let a_score = call_score_fn(score_fn, a);
        let b_score = call_score_fn(score_fn, b);
        a_score.total_cmp(&b_score)
    }
}

/// Container for a distinct life-form and all of its associated data.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Individual {
    /// Name or UUID of this individual.
    pub name: String,

    /// Number of individuals who died before this one,
    /// or None if this individual has not yet died.
    pub ascension: Option<u64>,

    /// Name of the environment that this individual lives in.
    pub environment: String,

    /// Name of the population that this individual belongs to.
    pub population: String,

    /// UUID of this individual's species.
    /// Mating may be restricted to individuals of the same species.
    pub species: String,

    /// Command line invocation of the controller program.
    pub controller: Vec<String>,

    /// Genetic parameters for this individual.
    #[serde(skip)]
    pub genome: OnceLock<Arc<[u8]>>,

    /// The environmental info dictionary. The environment updates this information.
    pub telemetry: HashMap<String, String>,

    /// The epigenetic info dictionary. The controller updates this information.
    pub epigenome: HashMap<String, String>,

    /// Reproductive fitness of this individual, as assessed by the environment.
    pub score: Option<String>,

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

    /// Custom / unofficial fields that are saved with the individual.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,

    /// Get the file path this individual was loaded from or saved to, or None
    /// if this individual has not touched the file system.
    #[serde(skip)]
    pub path: Option<PathBuf>,
}

impl Individual {
    /// Create a new individual.
    pub fn new(environment: &str, population: &str, controller: &[&str], genome: Box<[u8]>) -> Individual {
        Individual {
            name: uuid4(),
            ascension: None,
            environment: environment.to_string(),
            population: population.to_string(),
            species: uuid4(),
            controller: controller.iter().map(|arg| arg.to_string()).collect(),
            genome: OnceLock::from(Arc::from(genome)),
            telemetry: HashMap::new(),
            epigenome: HashMap::new(),
            score: None,
            generation: 0,
            parents: Vec::new(),
            children: Vec::new(),
            birth_date: String::new(),
            death_date: String::new(),
            extra: HashMap::new(),
            path: None,
        }
    }

    /// Get the genetic parameters for this individual. This loads the genome
    /// from file if necessary.
    pub fn genome(&self) -> Arc<[u8]> {
        self.genome
            .get_or_init(|| {
                let path = self.path.as_ref().expect("missing genome");
                // Safe to unwrap this file access, because we already accessed
                // this file at least once since the start of this program run.
                let mut file = BufReader::new(File::open(path).unwrap());
                file.skip_until(b'\0').unwrap();
                let mut data = vec![];
                file.read_to_end(&mut data).unwrap();
                Arc::from(data)
            })
            .clone()
    }

    /// Asexually reproduce an individual.
    ///
    /// Returns a pair of (individual, phenome)
    pub fn asex(&mut self, clone_genome: &CloneGenome) -> (Individual, Box<[u8]>) {
        let (genome, phenome) = clone_genome(&self.genome());
        let individual = Individual {
            name: uuid4(),
            ascension: None,
            environment: self.environment.clone(),
            population: self.population.clone(),
            species: self.species.clone(),
            controller: self.controller.clone(),
            genome: OnceLock::from(Arc::from(genome)),
            telemetry: HashMap::new(),
            epigenome: HashMap::new(),
            score: None,
            generation: self.generation + 1,
            parents: vec![self.name.clone()],
            children: vec![],
            birth_date: String::new(),
            death_date: String::new(),
            extra: HashMap::new(),
            path: None,
        };
        self.children.push(individual.name.clone());
        (individual, phenome)
    }

    /// Sexually reproduce two individuals.
    ///
    /// Returns a pair of (individual, phenome)
    pub fn sex(&mut self, other: &mut Individual, mate_genomes: &MateGenomes) -> (Individual, Box<[u8]>) {
        let (genome, phenome) = mate_genomes(&self.genome(), &other.genome());
        let individual = Individual {
            name: uuid4(),
            ascension: None,
            environment: self.environment.clone(),
            population: self.population.clone(),
            species: self.species.clone(),
            controller: self.controller.clone(),
            genome: OnceLock::from(Arc::from(genome)),
            telemetry: HashMap::new(),
            epigenome: HashMap::new(),
            score: None,
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
        (individual, phenome)
    }

    fn file_name(&self) -> String {
        format!("{}.indiv", self.name)
    }

    /// Save an individual to a file.
    ///
    /// Argument path is the directory to save in. Optional, use empty string
    /// for temporary file. The filename will be the individual's name with the
    /// ".indiv" file extension.
    ///
    /// Returns the file path of the saved individual.
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<&Path, std::io::Error> {
        let mut path: PathBuf = path.as_ref().into();
        // Fill in default path.
        if path.to_str() == Some("") {
            if let Some(save_file) = self.path.as_ref() {
                path = save_file.parent().unwrap().into();
            } else {
                path = std::env::temp_dir();
            }
        }
        // Load the genome from file before modifying the file system.
        self.genome();
        // Make the directory in case this is the first individual to be saved to it.
        if !path.exists() {
            std::fs::create_dir(&path)?;
        }
        // Make paths to temporary buffer and final file locations.
        let file_name = self.file_name();
        let mut temp = std::env::temp_dir();
        temp.push(format!("{}.tmp", file_name));
        path.push(file_name);
        //
        let file = File::create(&temp)?;
        let mut buf = BufWriter::new(file);
        serde_json::to_writer(&mut buf, self).unwrap();
        buf.write_all(b"\0")?;
        buf.write_all(&self.genome())?;
        let file = buf.into_inner()?; // flush the buffer
        file.sync_all()?; // push to disk
        std::fs::rename(&temp, &path)?; // move file into place
        self.path = Some(path);
        Ok(self.path.as_ref().unwrap())
    }

    /// Clone an individual and save it to file.
    ///
    /// Returns the new individual, which is equal to the input individual
    /// except that it is saved to a new file and its genome is not loaded.
    pub fn save_clone(&self, path: impl AsRef<Path>) -> Result<Individual, std::io::Error> {
        self.genome(); // Load the genome into memory before the Arc gets cloned.
        let mut clone = self.clone();
        clone.save(path)?;
        clone.genome.take();
        Ok(clone)
    }

    /// Safety: ensure that this individual's save file is up to date.
    unsafe fn copy_clone(&self, path: impl AsRef<Path>) -> Result<Individual, std::io::Error> {
        let old_path = self.path.as_ref().unwrap();
        let new_path = path.as_ref().join(self.file_name());
        std::fs::copy(old_path, &new_path)?;
        let mut clone = self.clone();
        clone.path = Some(new_path);
        Ok(clone)
    }

    /// Load a previously saved individual.
    pub fn load(path: impl AsRef<Path>) -> Result<Individual, std::io::Error> {
        let path = path.as_ref().to_path_buf();
        let mut file = BufReader::new(File::open(&path)?);
        let mut text = vec![];
        file.read_until(b'\0', &mut text)?;
        text.pop_if(|&mut char| char == b'\0');
        let mut individual: Individual = serde_json::from_slice(&text)?;
        individual.path = Some(path);
        Ok(individual)
    }

    /// Get all individuals saved in a directory.
    pub fn load_dir(path: impl AsRef<Path>) -> Result<Vec<Individual>, std::io::Error> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(vec![]);
        }
        let mut individuals = vec![];
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
                individuals.push(Individual::load(file.path())?);
            }
        }
        Ok(individuals)
    }

    /// Delete this individual and its associated save file.
    pub fn remove(self) -> Result<(), std::io::Error> {
        if let Some(path) = self.path.as_ref() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

/// A group of individuals that are stored together in a directory.
pub struct Population {
    path: PathBuf,

    replacement: Replacement,

    population_size: usize,

    leaderboard_size: usize,

    hall_of_fame_size: usize,

    score_fn: Option<Box<ScoreFn>>,

    ascension: u64,

    generation: u64,

    members: Vec<Individual>,

    waiting: Vec<Individual>,

    leaderboard: Vec<Individual>,

    hall_of_fame: Vec<Individual>,
}

/// Controls how a population replaces its members once it's full.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Replacement {
    /// Do not replace members. The population grows without bounds.
    Unbounded,

    /// Replace members at random.
    Random,

    /// Replace the oldest member.
    Oldest,

    /// Replace the lowest scoring member.
    Worst,

    /// Replace generations entirely and all at once.
    Generation,
}

#[derive(Serialize, Deserialize)]
struct PopulationMetadata {
    ascension: u64,
    generation: u64,
}

impl Population {
    pub fn new(
        path: impl AsRef<Path>,
        replacement: Replacement,
        population_size: usize,
        leaderboard_size: usize,
        hall_of_fame_size: usize,
        score_fn: Option<Box<ScoreFn>>,
    ) -> Result<Population, std::io::Error> {
        assert!(population_size > 0);
        let mut path = path.as_ref().to_path_buf();
        // Fill in empty path with temp dir.
        if path.to_str() == Some("") {
            path = std::env::temp_dir();
            path.push(format!("pop{:x}", rand::random_range(0..u64::MAX)));
        }
        let mut this = Population {
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
        this.load()?;
        Ok(this)
    }
    fn load(&mut self) -> Result<(), std::io::Error> {
        self.load_metadata()?;
        self.load_individuals()?;
        Ok(())
    }
    fn load_metadata(&mut self) -> Result<(), std::io::Error> {
        let path = self.get_metadata_path();
        if path.exists() {
            let metadata: PopulationMetadata = serde_json::from_slice(&std::fs::read(path)?).unwrap();
            self.ascension = metadata.ascension;
            self.generation = metadata.generation;
        }
        Ok(())
    }
    fn save_metadata(&self) -> Result<(), std::io::Error> {
        let path = self.get_metadata_path();
        let metadata = PopulationMetadata {
            ascension: self.ascension,
            generation: self.generation,
        };
        std::fs::write(path, serde_json::to_string(&metadata).unwrap())?;
        Ok(())
    }
    fn load_individuals(&mut self) -> Result<(), std::io::Error> {
        // First setup the file system.
        let members_path = self.get_members_path();
        let waiting_path = self.get_waiting_path();
        let leaderboard_path = self.get_leaderboard_path();
        let hall_of_fame_path = self.get_hall_of_fame_path();
        for path in [
            &self.path,
            &members_path,
            &waiting_path,
            &leaderboard_path,
            &hall_of_fame_path,
        ] {
            if !path.exists() {
                std::fs::create_dir(path)?;
            }
        }
        //
        self.members = Individual::load_dir(members_path)?;
        self.waiting = Individual::load_dir(waiting_path)?;
        self.leaderboard = Individual::load_dir(leaderboard_path)?;
        self.hall_of_fame = Individual::load_dir(hall_of_fame_path)?;
        //
        self.leaderboard.sort_by(compare_scores(self.score_fn.as_deref()));
        self.hall_of_fame.sort_by_key(|x| x.ascension.unwrap_or(u64::MAX));
        Ok(())
    }
    /// Get the path argument or a temporary directory.
    pub fn get_path(&self) -> &Path {
        &self.path
    }
    /// Get the replacement argument.
    pub fn get_replacement(&self) -> Replacement {
        self.replacement
    }
    /// Get the population_size argument.
    pub fn get_population_size(&self) -> usize {
        self.population_size
    }
    fn get_metadata_path(&self) -> PathBuf {
        self.path.join("population.json")
    }
    /// Get the current population's directory.
    fn get_members_path(&self) -> PathBuf {
        self.path.join("members")
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
    /// Get the number of cohorts of population_size that have been added.
    pub fn get_generation(&self) -> u64 {
        self.generation
    }
    /// Get the current members of the population.
    pub fn get_members(&self) -> &[Individual] {
        &self.members
    }
    /// Get the highest scoring individuals ever recorded. This is sorted
    /// descending by score, so that leaderboard\[0\] is the best individual.
    pub fn get_leaderboard(&self) -> &[Individual] {
        &self.leaderboard
    }
    /// Get the highest scoring individuals from each generation. This is sorted
    /// by ascension, so that hall_of_fame\[0\] is the oldest.
    pub fn get_hall_of_fame(&self) -> &[Individual] {
        &self.hall_of_fame
    }
    /// Add a new individual to this population.
    pub fn add(&mut self, mut individual: Individual) -> Result<(), std::io::Error> {
        //
        let ascension = self.ascension;
        individual.ascension = Some(ascension);
        self.ascension += 1;
        // Stage the individual in the waiting directory.
        let waiting_clone = individual.save_clone(self.get_waiting_path())?;
        // Make room in the current members directory for one more individual.
        match self.replacement {
            Replacement::Unbounded => {}
            Replacement::Generation => {}
            Replacement::Random => {
                while self.members.len() >= self.population_size {
                    let random_index = rand::random_range(0..self.members.len());
                    self.members.swap_remove(random_index).remove()?;
                }
            }
            Replacement::Worst => {
                let cmp = compare_scores(self.score_fn.as_deref());
                while self.members.len() >= self.population_size {
                    let (worst_index, _worst_individual) =
                        self.members.iter().enumerate().min_by(|a, b| cmp(a.1, b.1)).unwrap();
                    self.members.swap_remove(worst_index).remove()?;
                }
            }
            Replacement::Oldest => {
                while self.members.len() >= self.population_size {
                    let (oldest_index, _oldest_individual) = self
                        .members
                        .iter()
                        .enumerate()
                        .min_by_key(|(_index, individual)| individual.ascension)
                        .unwrap();
                    self.members.swap_remove(oldest_index).remove()?;
                }
            }
        }
        // Save the individual into the current generation.
        match self.replacement {
            Replacement::Unbounded | Replacement::Random | Replacement::Worst | Replacement::Oldest => {
                let waiting_path = waiting_clone.path.as_ref().unwrap();
                let mut member_path = self.get_members_path();
                member_path.push(individual.file_name());
                std::fs::copy(waiting_path, &member_path)?;
                individual.path = Some(member_path);
                self.members.push(individual);
            }
            Replacement::Generation => {} // Does not save to the current generation.
        }
        self.waiting.push(waiting_clone);
        // Cycle the next generation into place, and update bookkeeping.
        if self.waiting.len() >= self.population_size {
            self.rollover()?;
        }
        Ok(())
    }
    fn rollover(&mut self) -> Result<(), std::io::Error> {
        self.rollover_leaderboard()?;
        self.rollover_hall_of_fame()?;
        self.rollover_generation()?;
        Ok(())
    }
    fn rollover_leaderboard(&mut self) -> Result<(), std::io::Error> {
        let score_fn = self.score_fn.as_deref();
        // Sort together the existing leaderboard and the new contenders.
        if !self.leaderboard.is_empty() && self.leaderboard.len() >= self.leaderboard_size {
            let min_score = call_score_fn(score_fn, self.leaderboard.last().unwrap());
            self.leaderboard.extend(
                self.waiting
                    .iter()
                    .filter(|individual| call_score_fn(score_fn, individual) > min_score)
                    .cloned(),
            );
        } else {
            self.leaderboard.extend_from_slice(&self.waiting); // clone
        }
        self.leaderboard.sort_by(compare_scores(score_fn));
        // Move new winners to the leaderboard directory.
        let leaderboard_path = self.get_leaderboard_path();
        for individual in &mut self.leaderboard[..self.leaderboard_size] {
            let old_path = individual.path.as_ref().unwrap();
            if !old_path.starts_with(&leaderboard_path) {
                let new_path = leaderboard_path.join(individual.file_name());
                std::fs::copy(old_path, &new_path)?;
                individual.path = Some(new_path);
            }
        }
        // Remove low performing individuals from the leaderboard directory.
        for individual in self.leaderboard.drain(self.leaderboard_size..) {
            if individual.path.as_ref().unwrap().starts_with(&leaderboard_path) {
                individual.remove()?;
            }
        }
        Ok(())
    }
    fn rollover_hall_of_fame(&mut self) -> Result<(), std::io::Error> {
        let hall_of_fame_path = self.get_hall_of_fame_path();
        // Find the highest scoring individuals in the new generation.
        self.waiting.sort_by(compare_scores(self.score_fn.as_deref()));
        let winners = &mut self.waiting[..self.hall_of_fame_size];
        winners.sort_unstable_by_key(|individual| individual.ascension);
        // Copy the inductees into the hall_of_fame directory.
        for individual in winners {
            unsafe {
                self.hall_of_fame.push(individual.copy_clone(&hall_of_fame_path)?);
            }
        }
        Ok(())
    }
    fn rollover_generation(&mut self) -> Result<(), std::io::Error> {
        self.generation += 1;
        if self.replacement == Replacement::Generation {
            // Delete the current generation.
            let members_path = self.get_members_path();
            std::fs::remove_dir_all(&members_path)?;
            self.members.clear();
            // Move the next generation into place.
            let waiting_path = self.get_waiting_path();
            std::fs::rename(&waiting_path, &members_path)?;
            std::fs::create_dir(waiting_path)?;
            std::mem::swap(&mut self.members, &mut self.waiting);
            self.save_metadata()?;
            // Update the moved individual's path field.
            for individual in &mut self.members {
                individual.path = Some(members_path.join(individual.file_name()));
            }
        } else {
            self.save_metadata()?;
            // Discard the old generation.
            for individual in self.waiting.drain(..) {
                individual.remove()?;
            }
        }
        Ok(())
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

type MateSelection = dyn mate_selection::MateSelection<rand::rngs::ThreadRng>;

pub struct Evo(Mutex<Inner>);

struct Inner {
    population: Population,

    generation: u64,

    parents: Vec<(Arc<Individual>, Arc<Individual>)>,
}

impl Evo {
    pub fn new(_population: Population, _mate_selection: Box<MateSelection>) -> Self {
        // Self {}
        todo!()
    }
}
impl Evolution for Evo {
    fn spawn(&self) -> (Individual, Box<[u8]>) {
        let mut inner = self.0.lock().unwrap();
        if inner.generation != inner.population.generation {
            inner.parents.clear();
            inner.generation = inner.population.generation;
        }

        if inner.parents.is_empty() {
            let members = inner.population.get_members();
            let score_fn = inner.population.score_fn.as_deref();
            let _scores = members
                .iter()
                .map(|indivdiual| call_score_fn(score_fn, indivdiual))
                .collect::<Vec<f64>>();
            let _buffer = inner.population.get_population_size();
            // let pairs = mate_selection.pairs(scores, buffer);
        }

        todo!()
    }
    fn death(&self, individual: Individual) {
        self.0.lock().unwrap().population.add(individual).unwrap()
    }
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
        let mut indiv1 = Individual::new("foo", "bar", &["ctrl", "prog"], Box::new(*b"beepboop"));
        indiv1.extra.insert("X".into(), "Y".into());
        let path = dbg!(indiv1.save(std::env::temp_dir())).unwrap();
        let indiv2 = Individual::load(path).unwrap();
        indiv2.genome();
        assert_eq!(indiv1, indiv2);
    }
    #[test]
    fn pop_save_load() {
        let mut pop1 = Population::new("", Replacement::Unbounded, 10, 3, 1, None).unwrap();

        for _ in 0..30 {
            let mut genome = Box::new(*b"beepboop");
            rand::fill(&mut genome[..]);
            let indiv = Individual::new("foo", "bar", &["ctrl", "prog"], genome);
            pop1.add(indiv).unwrap();
        }
        dbg!(pop1.get_members());
        pop1.save_metadata().unwrap();
        let pop2 = Population::new(pop1.get_path(), Replacement::Unbounded, 10, 3, 1, None).unwrap();
        assert_eq!(pop1.get_path(), pop2.get_path());
        assert_eq!(pop1.get_ascension(), pop2.get_ascension());
        assert_eq!(pop1.get_generation(), pop2.get_generation());
        fn cmp_indiv(individuals: &[Individual]) -> Vec<(Option<PathBuf>, Arc<[u8]>)> {
            let mut stubs = individuals
                .iter()
                .map(|stub| (stub.path.clone(), stub.genome()))
                .collect::<Vec<(Option<PathBuf>, Arc<[u8]>)>>();
            stubs.sort();
            stubs
        }
        assert_eq!(cmp_indiv(pop1.get_members()), cmp_indiv(pop2.get_members()));
        assert_eq!(cmp_indiv(pop1.get_leaderboard()), cmp_indiv(pop2.get_leaderboard()));
        assert_eq!(cmp_indiv(pop1.get_hall_of_fame()), cmp_indiv(pop2.get_hall_of_fame()));
    }
}
