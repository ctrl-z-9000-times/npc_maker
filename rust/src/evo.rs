//! Evolutionary algorithms and supporting tools.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Error, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Generate a universally unique name. This will never return the same name twice.
fn uuid4() -> String {
    let uuid: u128 = rand::random();
    format!("{uuid:032X}")
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

    /// Name or UUID of this individual's species.
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

    /// The file path this individual was loaded from or saved to, or None
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
    pub fn asex(&mut self, child_genome: &[u8]) -> Individual {
        let individual = Individual {
            name: uuid4(),
            ascension: None,
            environment: self.environment.clone(),
            population: self.population.clone(),
            species: self.species.clone(),
            controller: self.controller.clone(),
            genome: OnceLock::from(Arc::from(child_genome)),
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
        individual
    }

    /// Sexually reproduce two individuals.
    pub fn sex(&mut self, other: &mut Individual, child_genome: &[u8]) -> Individual {
        let individual = Individual {
            name: uuid4(),
            ascension: None,
            environment: self.environment.clone(),
            population: self.population.clone(),
            species: self.species.clone(),
            controller: self.controller.clone(),
            genome: OnceLock::from(Arc::from(child_genome)),
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
        individual
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
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<&Path, Error> {
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
        self.genome.take(); // free the genome
        Ok(self.path.as_ref().unwrap())
    }

    /// Load a previously saved individual.
    pub fn load(path: impl AsRef<Path>) -> Result<Individual, Error> {
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
    pub fn load_dir(path: impl AsRef<Path>) -> Result<Vec<Individual>, Error> {
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

    /// Remove this individual and its associated save file.
    pub fn delete(self) -> Result<(), Error> {
        if let Some(path) = self.path {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Delete the individual if this is the last reference to it.
    pub fn drop(this: Arc<Mutex<Self>>) -> Result<(), Error> {
        if let Some(mutex) = Arc::into_inner(this) {
            let individual = mutex.into_inner().unwrap();
            individual.delete()?;
        }
        Ok(())
    }
}

/// Individuals may have custom scores functions with this type signature.
///
/// By default the npc_maker will parse the individual's score into a single
/// floating point number, with a default of -inf for missing or invalid scores.
pub type ScoreFn = dyn Fn(&Individual) -> f64;

const DEFAULT_SCORE: f64 = f64::NEG_INFINITY;

fn call_score_fn(score_fn: Option<&ScoreFn>, individual: &Arc<Mutex<Individual>>) -> f64 {
    let individual = individual.lock().unwrap();
    if let Some(score_fn) = score_fn {
        score_fn(&individual)
    } else if let Some(score) = &individual.score {
        score.parse().unwrap_or(DEFAULT_SCORE)
    } else {
        DEFAULT_SCORE
    }
}

fn compare_scores(
    score_fn: Option<&ScoreFn>,
) -> impl Fn(&Arc<Mutex<Individual>>, &Arc<Mutex<Individual>>) -> std::cmp::Ordering {
    move |a, b| {
        let a_score = call_score_fn(score_fn, a);
        let b_score = call_score_fn(score_fn, b);
        a_score.total_cmp(&b_score).reverse()
    }
}

// TOOD: Consider introducing a mutex lock into the population so that
// Population.add() is immutable. This would allow finer grained locking.
// Then move the file-system tasks out of the mutex-locked critical section.
//
// Psuedo Code:
//
// fn add(&self, individual) {
//      1) individual.save()
//      2) lock mutex
//      3) reckon the population
//      4) release mutex
//      5) delete individuals
//

/// A group of individuals.
///
/// This manages an evolving population of individuals, featuring:
/// * Serveral strategies for replacing individuals with new ones,
/// * Persistant populations that are saved to file,
/// * And a Leaderboard and Hall of Fame.
pub struct Population {
    path: PathBuf,

    replacement: Replacement,

    population_size: usize,

    leaderboard_size: usize,

    hall_of_fame_size: usize,

    score_fn: Option<Box<ScoreFn>>,

    ascension: u64,

    generation: u64,

    members: Vec<Arc<Mutex<Individual>>>,

    waiting: Vec<Arc<Mutex<Individual>>>,

    leaderboard: Vec<Arc<Mutex<Individual>>>,

    hall_of_fame: Vec<Arc<Mutex<Individual>>>,
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
    members: Vec<String>,
    waiting: Vec<String>,
    leaderboard: Vec<String>,
    hall_of_fame: Vec<String>,
}

impl Population {
    ///
    pub fn new(
        path: impl AsRef<Path>,
        replacement: Replacement,
        population_size: usize,
        leaderboard_size: usize,
        hall_of_fame_size: usize,
        score_fn: Option<Box<ScoreFn>>,
    ) -> Result<Population, Error> {
        let mut path = path.as_ref().to_path_buf();
        // Fill in empty path with temp dir.
        if path.to_str() == Some("") {
            path = std::env::temp_dir();
            path.push(format!("pop{:x}", rand::random_range(0..u64::MAX)));
        }
        if !path.exists() {
            std::fs::create_dir(&path)?;
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
    fn save(&self) -> Result<(), Error> {
        let get_name = |indiv: &Arc<Mutex<Individual>>| indiv.lock().unwrap().name.clone();
        let metadata = PopulationMetadata {
            ascension: self.ascension,
            generation: self.generation,
            members: self.members.iter().map(get_name).collect(),
            waiting: self.waiting.iter().map(get_name).collect(),
            leaderboard: self.leaderboard.iter().map(get_name).collect(),
            hall_of_fame: self.hall_of_fame.iter().map(get_name).collect(),
        };
        let path = self.get_metadata_path();
        std::fs::write(path, serde_json::to_vec(&metadata).unwrap())?;
        Ok(())
    }
    fn load(&mut self) -> Result<(), Error> {
        let path = self.get_metadata_path();
        if !path.exists() {
            return Ok(());
        }
        let metadata: PopulationMetadata = serde_json::from_slice(&std::fs::read(&path)?).unwrap();
        self.ascension = metadata.ascension;
        self.generation = metadata.generation;
        //
        let individuals: HashMap<String, Arc<Mutex<Individual>>> = Individual::load_dir(&self.path)?
            .into_iter()
            .map(|individual| (individual.name.to_string(), Arc::from(Mutex::from(individual))))
            .collect();
        let lookup = |individual: &String| individuals.get(individual).unwrap().clone();
        self.members = metadata.members.iter().map(lookup).collect();
        self.waiting = metadata.waiting.iter().map(lookup).collect();
        self.leaderboard = metadata.leaderboard.iter().map(lookup).collect();
        self.hall_of_fame = metadata.hall_of_fame.iter().map(lookup).collect();
        // Sort the historical data to enforce invariants.
        self.leaderboard.sort_by(compare_scores(self.score_fn.as_deref()));
        self.hall_of_fame
            .sort_by_key(|x| x.lock().unwrap().ascension.unwrap_or(u64::MAX));
        Ok(())
    }
    /// Get the path argument or a temporary directory.
    pub fn get_path(&self) -> &Path {
        &self.path
    }
    fn get_metadata_path(&self) -> PathBuf {
        self.path.join("population.json")
    }
    /// Get the replacement argument.
    pub fn get_replacement(&self) -> Replacement {
        self.replacement
    }
    /// Get the population_size argument.
    pub fn get_population_size(&self) -> usize {
        self.population_size
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
    pub fn get_members(&self) -> &[Arc<Mutex<Individual>>] {
        &self.members
    }
    /// Get the highest scoring individuals ever recorded. This is sorted
    /// descending by score, so that leaderboard\[0\] is the best individual.
    pub fn get_leaderboard(&self) -> &[Arc<Mutex<Individual>>] {
        &self.leaderboard
    }
    /// Get the highest scoring individuals from each generation. This is sorted
    /// by ascension, so that hall_of_fame\[0\] is the oldest.
    pub fn get_hall_of_fame(&self) -> &[Arc<Mutex<Individual>>] {
        &self.hall_of_fame
    }
    /// Add a new individual to this population.
    pub fn add(&mut self, mut individual: Individual) -> Result<Arc<Mutex<Individual>>, Error> {
        debug_assert!(individual.ascension.is_none());
        individual.ascension = Some(self.ascension);
        self.ascension += 1;
        //
        individual.save(&self.path)?;
        let individual = Arc::from(Mutex::from(individual));
        // Make room in the current members list for another individual.
        match self.replacement {
            Replacement::Unbounded => {}
            Replacement::Generation => {}
            Replacement::Random => {
                while !self.members.is_empty() && self.members.len() >= self.population_size {
                    let random_index = rand::random_range(0..self.members.len());
                    let random_individual = self.members.swap_remove(random_index);
                    Individual::drop(random_individual)?;
                }
            }
            Replacement::Worst => {
                let compare_scores = compare_scores(self.score_fn.as_deref());
                while !self.members.is_empty() && self.members.len() >= self.population_size {
                    let (worst_index, _worst_individual) = self
                        .members
                        .iter()
                        .enumerate()
                        .min_by(|a, b| compare_scores(a.1, b.1))
                        .unwrap();
                    let worst_individual = self.members.swap_remove(worst_index);
                    Individual::drop(worst_individual)?;
                }
            }
            Replacement::Oldest => {
                while !self.members.is_empty() && self.members.len() >= self.population_size {
                    let (oldest_index, _oldest_individual) = self
                        .members
                        .iter()
                        .enumerate()
                        .min_by_key(|(_index, individual)| individual.lock().unwrap().ascension)
                        .unwrap();
                    let oldest_individual = self.members.swap_remove(oldest_index);
                    Individual::drop(oldest_individual)?;
                }
            }
        }
        // Save the individual into the current generation.
        match self.replacement {
            Replacement::Unbounded | Replacement::Random | Replacement::Worst | Replacement::Oldest => {
                self.members.push(individual.clone());
            }
            Replacement::Generation => {}
        }
        // Stage the individual for the next generation and bookkeeping.
        self.waiting.push(individual.clone());
        if self.waiting.len() >= self.population_size {
            self.rollover()?;
        }
        Ok(individual)
    }
    ///
    pub fn rollover(&mut self) -> Result<(), Error> {
        self.rollover_leaderboard()?;
        self.rollover_hall_of_fame()?;
        self.rollover_generation()?;
        self.save()?;
        Ok(())
    }
    fn rollover_leaderboard(&mut self) -> Result<(), Error> {
        if self.leaderboard_size == 0 {
            return Ok(());
        }
        let score_fn = self.score_fn.as_deref();
        let min_score = if self.leaderboard.len() >= self.leaderboard_size {
            call_score_fn(score_fn, self.leaderboard.last().unwrap())
        } else {
            f64::NEG_INFINITY
        };
        // Sort together the existing leaderboard and the new contenders.
        self.leaderboard.extend(
            self.waiting
                .iter()
                .filter(|individual| call_score_fn(score_fn, individual) > min_score)
                .cloned(),
        );
        // Use stable sort to preserve ascension ordering.
        self.leaderboard.sort_by(compare_scores(score_fn));
        // Remove low performing individuals from the leaderboard directory.
        if self.leaderboard.len() > self.leaderboard_size {
            for individual in self.leaderboard.drain(self.leaderboard_size..) {
                Individual::drop(individual)?;
            }
        }
        Ok(())
    }
    fn rollover_hall_of_fame(&mut self) -> Result<(), Error> {
        // Find the highest scoring individuals in the new generation.
        let n = self.hall_of_fame_size.min(self.waiting.len() - 1);
        // This should be a stable sort but std does not support it.
        self.waiting
            .select_nth_unstable_by(n, compare_scores(self.score_fn.as_deref()));
        let winners = &mut self.waiting[..n];
        winners.sort_unstable_by_key(|individual| individual.lock().unwrap().ascension);
        self.hall_of_fame.extend_from_slice(winners);
        Ok(())
    }
    fn rollover_generation(&mut self) -> Result<(), Error> {
        self.generation += 1;
        // Move the next generation into place.
        if self.replacement == Replacement::Generation {
            std::mem::swap(&mut self.members, &mut self.waiting);
        }
        // Discard the old generation.
        for individual in self.waiting.drain(..) {
            Individual::drop(individual)?;
        }
        Ok(())
    }
}

/// Interface for evolutionary algorithms.
pub trait API {
    /// Get a new individual to be born into an environment.
    ///
    /// Returns an individual and a genome for the controller, which may differ
    /// from the individual's genome.
    fn spawn(&self) -> (Individual, Box<[u8]>);

    /// Notification of an individual's death.
    fn death(&self, individual: Individual);
}

pub struct Evolution {
    inner: Mutex<Inner>,

    mate_selection: Box<MateSelection>,

    mate_genomes: Box<GenomeSex>,
}

struct Inner {
    population: Population,

    generation: u64,

    parents: Vec<(Arc<Mutex<Individual>>, Arc<Mutex<Individual>>)>,
}

///
pub type MateSelection = dyn mate_selection::MateSelection<rand::rngs::ThreadRng>;

/// Callback for asexually reproducing a genome.
///
/// Returns a pair of (genome, phenome)
pub type GenomeAsex = dyn Fn(&[u8]) -> (Box<[u8]>, Box<[u8]>);

/// Callback for sexually reproducing two genomes.
///
/// Returns a pair of (genome, phenome)
pub type GenomeSex = dyn Fn(&[u8], &[u8]) -> (Box<[u8]>, Box<[u8]>);

impl Evolution {
    ///
    pub fn new(
        population: Population,
        mate_selection: Box<MateSelection>,
        mate_genomes: Box<GenomeSex>,
    ) -> Result<Self, Error> {
        let generation = population.get_generation();
        Ok(Self {
            inner: Mutex::from(Inner {
                population,
                generation,
                parents: vec![],
            }),
            mate_selection,
            mate_genomes,
        })
    }
    pub fn into_inner(self) -> Population {
        self.inner.into_inner().unwrap().population
    }
    pub fn get_generation(&self) -> u64 {
        self.inner.lock().unwrap().generation
    }
}
impl API for Evolution {
    fn spawn(&self) -> (Individual, Box<[u8]>) {
        let rng = &mut rand::rng();
        // Lock and unpack this structure.
        let mut inner = self.inner.lock().unwrap();
        let Inner {
            population,
            generation,
            parents,
        } = &mut *inner;
        // Check for rollover event.
        if *generation != population.get_generation() {
            parents.clear();
            *generation = population.get_generation();
        }
        // Refill parents buffer.
        if parents.is_empty() {
            let num_pairs = if population.get_replacement() == Replacement::Generation {
                population.get_population_size()
            } else {
                1
            };
            let members = population.get_members();
            let score_fn = |indivdiual| call_score_fn(population.score_fn.as_deref(), indivdiual);
            let scores = members.iter().map(score_fn).collect::<Vec<f64>>();
            let pairs = self.mate_selection.pairs(rng, num_pairs, scores);
            let members = population.get_members();
            for [index1, index2] in pairs {
                let parent1 = members[index1].clone();
                let parent2 = members[index2].clone();
                parents.push((parent1, parent2));
            }
        }
        // Get and mate parents.
        let (parent1, parent2) = parents.pop().unwrap();
        drop(inner);
        if Arc::as_ptr(&parent1) == Arc::as_ptr(&parent2) {
            let mut parent1 = parent1.lock().unwrap();
            let (genome, phenome) = (self.mate_genomes)(&parent1.genome(), &parent1.genome());
            (parent1.asex(&genome), phenome)
        } else {
            let mut parent1 = parent1.lock().unwrap();
            let mut parent2 = parent2.lock().unwrap();
            let (genome, phenome) = (self.mate_genomes)(&parent1.genome(), &parent2.genome());
            (parent1.sex(&mut parent2, &genome), phenome)
        }
    }
    fn death(&self, individual: Individual) {
        let mut inner = self.inner.lock().unwrap();
        inner.population.add(individual).unwrap();
    }
}

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
        indiv1.genome();
        indiv2.genome();
        assert_eq!(indiv1, indiv2);
    }
    #[test]
    fn pop_save_load() {
        let mut pop1 = Population::new("", Replacement::Generation, 10, 3, 2, None).unwrap();

        for _ in 0..30 {
            let mut genome = Box::new(*b"beepboop");
            rand::fill(&mut genome[..]);
            let mut indiv = Individual::new("foo", "bar", &["ctrl", "prog"], genome);
            indiv.score = Some(rand::random::<f64>().to_string());
            pop1.add(indiv).unwrap();
        }
        pop1.save().unwrap();
        let pop2 = Population::new(pop1.get_path(), Replacement::Generation, 10, 3, 2, None).unwrap();
        assert_eq!(pop1.get_path(), pop2.get_path());
        assert_eq!(pop1.get_ascension(), pop2.get_ascension());
        assert_eq!(pop1.get_generation(), pop2.get_generation());
        fn cmp_indiv(individuals: &[Arc<Mutex<Individual>>]) -> Vec<(Option<PathBuf>, Arc<[u8]>)> {
            let mut stubs = individuals
                .iter()
                .map(|stub| {
                    let stub = stub.lock().unwrap();
                    (stub.path.clone(), stub.genome())
                })
                .collect::<Vec<(Option<PathBuf>, Arc<[u8]>)>>();
            stubs.sort();
            stubs
        }
        assert_eq!(cmp_indiv(pop1.get_members()), cmp_indiv(pop2.get_members()));
        assert_eq!(cmp_indiv(pop1.get_leaderboard()), cmp_indiv(pop2.get_leaderboard()));
        assert_eq!(cmp_indiv(pop1.get_hall_of_fame()), cmp_indiv(pop2.get_hall_of_fame()));
        assert_eq!(pop2.get_members().len(), 10);
        assert_eq!(pop2.get_leaderboard().len(), 3);
        assert_eq!(pop2.get_hall_of_fame().len(), 6);
    }
    #[test]
    fn evo() {
        fn new_genome() -> Box<[u8]> {
            rand::random_iter::<u8>()
                .take(10)
                .collect::<Vec<u8>>()
                .into_boxed_slice()
        }
        fn mate_fn(a: &[u8], b: &[u8]) -> (Box<[u8]>, Box<[u8]>) {
            let n = a.len();
            let crossover = rand::random_range(0..n);
            let mut c = vec![];
            c.extend_from_slice(&a[0..crossover]);
            c.extend_from_slice(&b[crossover..n]);
            if rand::random_bool(0.5) {
                c[rand::random_range(0..n)] = rand::random();
            }
            (c.clone().into_boxed_slice(), c.into_boxed_slice())
        }
        fn eval(a: &[u8], b: &[u8]) -> String {
            let abs_dif = a.iter().zip(b).map(|(&x, &y)| (x as f64 - y as f64).abs()).sum::<f64>();
            (-abs_dif).to_string()
        }
        let target_genome = new_genome();
        let seed_genome = new_genome();
        let seed_score = eval(&seed_genome, &target_genome);
        let mut seed = Individual::new("", "", &[], seed_genome);
        seed.score = Some(seed_score);
        let mut pop = Population::new("", Replacement::Generation, 100, 3, 1, None).unwrap();
        pop.add(seed).unwrap();
        pop.rollover().unwrap();
        let evo = Evolution::new(pop, Box::new(mate_selection::RankedExponential(5)), Box::new(mate_fn)).unwrap();
        while evo.inner.lock().unwrap().population.get_generation() < 20 {
            let (mut x, y) = evo.spawn();
            x.score = Some(eval(&y, &target_genome));
            evo.death(x);
        }
        for indiv in evo.inner.lock().unwrap().population.get_hall_of_fame() {
            println!("{}", indiv.lock().unwrap().score.as_ref().unwrap());
        }
        let best = evo.inner.lock().unwrap().population.get_leaderboard()[0].clone();
        assert!(best.lock().unwrap().score.as_ref().unwrap().parse::<f64>().unwrap() > -100.0);
    }
}
