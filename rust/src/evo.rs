use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

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
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<&Path, std::io::Error> {
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
        let file = File::create(&temp)?;
        let mut buf = std::io::BufWriter::new(file);
        serde_json::to_writer(&mut buf, self).unwrap();
        buf.write_all(b"\0")?;
        buf.write_all(&self.genome)?;
        let file = buf.into_inner()?; // flush the buffer
        file.sync_all()?; // push to disk
        std::fs::rename(&temp, &path)?; // move file into place
        self.path = Some(path);
        Ok(self.path.as_ref().unwrap())
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

/// Individuals may have custom scores functions with this type signature.
///
/// By default the npc_maker will parse the individual's score into a single
/// floating point number.
pub type ScoreFn = Box<dyn Fn(&Individual) -> f64>;

fn unwrap_score_fn(score_fn: Option<ScoreFn>) -> ScoreFn {
    score_fn.unwrap_or(Box::new(|individual: &Individual| {
        individual.score.parse().unwrap_or(f64::NEG_INFINITY)
    }))
}

/// Handle to an Individual that is stored on file.
///
/// Stubs contain only the critical data for the evolutionary algorithm, and the
/// file path where the rest of the individual's data is stored.
#[derive(Debug, Clone, PartialEq)]
pub struct Stub {
    pub path: PathBuf,
    pub score: f64,
    pub ascension: u64,
}

impl Stub {
    pub fn new(path: impl AsRef<Path>, score_fn: Option<ScoreFn>) -> Result<Stub, std::io::Error> {
        let path = path.as_ref();
        let score_fn = unwrap_score_fn(score_fn);
        let individual = Individual::load(path)?;
        let score = score_fn(&individual);
        Ok(Stub {
            path: path.into(),
            score,
            ascension: individual.ascension.unwrap_or(u64::MAX),
        })
    }
    pub fn load(&self) -> Result<Individual, std::io::Error> {
        Individual::load(&self.path)
    }
    /// Argument path is a directory to copy this individual to.
    /// Returns a handle to the new copy.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<Stub, std::io::Error> {
        let path = path.as_ref();
        if !path.exists() {
            std::fs::create_dir(path)?;
        }
        let path = path.join(self.path.file_name().unwrap());
        std::fs::copy(&self.path, &path)?;
        Ok(Stub { path, ..*self })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn score(&self) -> f64 {
        self.score
    }
    pub fn ascension(&self) -> u64 {
        self.ascension
    }
}

/// Get the paths of all saved individuals in the directory.
fn scan_dir(path: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    if !path.exists() {
        std::fs::create_dir(path)?;
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
            individuals.push(file.path());
        }
    }
    Ok(individuals)
}

/// A group of individuals that are stored together in a directory.
pub struct Population {
    path: PathBuf,

    replacement: Replacement,

    population_size: usize,

    leaderboard_size: usize,

    hall_of_fame_size: usize,

    score_fn: ScoreFn,

    ascension: u64,

    generation: u64,

    members: Vec<Stub>,

    waiting: Vec<Stub>,

    leaderboard: Vec<Stub>,

    hall_of_fame: Vec<Stub>,
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
        score_fn: Option<ScoreFn>,
    ) -> Result<Self, std::io::Error> {
        let mut path: PathBuf = path.as_ref().into();
        // Fill in empty path with temp dir.
        if path.to_str() == Some("") {
            path = std::env::temp_dir();
            path.push(format!("pop{:x}", rand::random_range(0..u64::MAX)));
        }
        let mut this = Self {
            path,
            replacement,
            population_size,
            leaderboard_size,
            hall_of_fame_size,
            score_fn: unwrap_score_fn(score_fn),
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
        self.load_members()?;
        self.load_waiting()?;
        self.load_leaderboard()?;
        self.load_hall_of_fame()?;
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
    fn load_stubs(&self, path: &Path) -> Result<Vec<Stub>, std::io::Error> {
        let contents = scan_dir(path)?;
        let mut stubs = Vec::with_capacity(contents.len());
        for path in contents {
            let individual = Individual::load(&path)?;
            stubs.push(Stub {
                path,
                score: (self.score_fn)(&individual),
                ascension: individual.ascension.unwrap(),
            })
        }
        Ok(stubs)
    }
    fn load_members(&mut self) -> Result<(), std::io::Error> {
        self.members = self.load_stubs(&self.get_members_path())?;
        Ok(())
    }
    fn load_waiting(&mut self) -> Result<(), std::io::Error> {
        self.waiting = self.load_stubs(&self.get_waiting_path())?;
        Ok(())
    }
    fn load_leaderboard(&mut self) -> Result<(), std::io::Error> {
        self.leaderboard = self.load_stubs(&self.get_leaderboard_path())?;
        self.leaderboard.sort_by(|a, b| a.score.total_cmp(&b.score));
        Ok(())
    }
    fn load_hall_of_fame(&mut self) -> Result<(), std::io::Error> {
        self.hall_of_fame = self.load_stubs(&self.get_hall_of_fame_path())?;
        self.hall_of_fame.sort_by_key(|individual| individual.ascension);
        Ok(())
    }
    /// Get the path argument or a temporary directory.
    pub fn get_path(&self) -> &Path {
        &self.path
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
    /// Get the number of generations that have completely passed.
    pub fn get_generation(&self) -> u64 {
        self.generation
    }
    /// Get the current members of the population.
    pub fn get_members(&self) -> &[Stub] {
        &self.members
    }
    /// Get the highest scoring individuals ever recorded. This is sorted
    /// descending by score, so that leaderboard[0] is the best individual.
    pub fn get_leaderboard(&self) -> &[Stub] {
        &self.leaderboard
    }
    /// Get the highest scoring individuals from each generation. This is sorted
    /// by ascension, so that hall_of_fame[0] is the oldest.
    pub fn get_hall_of_fame(&self) -> &[Stub] {
        &self.hall_of_fame
    }
    /// Add a new individual to this population.
    pub fn add(&mut self, mut individual: Individual) -> Result<(), std::io::Error> {
        let ascension = self.ascension;
        individual.ascension = Some(ascension);
        self.ascension += 1;
        // Stage the individual in the waiting directory.
        let score = (self.score_fn)(&individual);
        self.waiting.push(Stub {
            path: individual.save(self.get_waiting_path())?.into(),
            score,
            ascension,
        });
        // Make room in the current generation for a new member.
        if self.members.len() >= self.population_size {
            match self.replacement {
                Replacement::Unbounded => {}
                Replacement::Random => {
                    let random_index = rand::random_range(0..self.members.len());
                    self.members.swap_remove(random_index);
                }
                Replacement::Worst => {
                    let (worst_index, _worst_individual) = self
                        .members
                        .iter()
                        .enumerate()
                        .min_by(|a, b| a.1.score.total_cmp(&b.1.score))
                        .unwrap();
                    self.members.swap_remove(worst_index);
                }
                Replacement::Oldest => {
                    let (oldest_index, _oldest_individual) = self
                        .members
                        .iter()
                        .enumerate()
                        .min_by_key(|(_index, individual)| individual.ascension)
                        .unwrap();
                    self.members.swap_remove(oldest_index);
                }
                Replacement::Generation => {}
            }
        }
        // Save directly to the current generation.
        match self.replacement {
            Replacement::Unbounded | Replacement::Random | Replacement::Worst | Replacement::Oldest => {
                self.members.push(Stub {
                    path: individual.save(self.get_members_path())?.into(),
                    score,
                    ascension,
                });
            }
            Replacement::Generation => {}
        }
        // Cycle the next generation into place, and update bookkeeping.
        if self.waiting.len() >= self.population_size {
            self.rollover().unwrap();
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
        let leaderboard_path = self.get_leaderboard_path();
        // Sort together the existing leaderboard and the new contenders.
        self.leaderboard.reserve(self.waiting.len());
        self.leaderboard.extend_from_slice(&self.waiting);
        self.leaderboard.sort_by(|a, b| a.score.total_cmp(&b.score));
        // Move new winners to the leaderboard directory.
        for individual in &mut self.leaderboard[..self.leaderboard_size] {
            if !individual.path.starts_with(&leaderboard_path) {
                *individual = individual.save(&leaderboard_path)?;
            }
        }
        // Remove low performing individuals from the leaderboard directory.
        for individual in self.leaderboard.drain(self.leaderboard_size..) {
            if individual.path.starts_with(&leaderboard_path) {
                std::fs::remove_file(&individual.path)?;
            }
        }
        Ok(())
    }
    fn rollover_hall_of_fame(&mut self) -> Result<(), std::io::Error> {
        let hall_of_fame_path = self.get_hall_of_fame_path();
        // Find the highest scoring individuals in the new generation.
        self.waiting.sort_by(|a, b| a.score.total_cmp(&b.score).reverse());
        let winners = &mut self.waiting[..self.hall_of_fame_size];
        winners.sort_unstable_by_key(|individual| individual.ascension);
        for individual in winners {
            self.hall_of_fame.push(individual.save(&hall_of_fame_path)?);
        }
        Ok(())
    }
    fn rollover_generation(&mut self) -> Result<(), std::io::Error> {
        self.generation += 1;
        if self.replacement == Replacement::Generation {
            // Swap the waiting and members directories.
            let members_path = self.get_members_path();
            let waiting_path = self.get_waiting_path();
            let swap_path = self.get_path().join(".swap");
            std::fs::rename(&members_path, &swap_path)?;
            std::fs::rename(&waiting_path, &members_path)?;
            std::fs::rename(&swap_path, &waiting_path)?;
            self.save_metadata()?;
            // Swap the old and new generations and update their stubs.
            std::mem::swap(&mut self.members, &mut self.waiting);
            for individual in &mut self.members {
                individual.path = members_path.join(individual.path.file_name().unwrap());
            }
            for individual in &mut self.waiting {
                individual.path = waiting_path.join(individual.path.file_name().unwrap());
            }
        } else {
            self.save_metadata()?;
        }
        // Discard the old generation.
        for individual in self.waiting.drain(..) {
            std::fs::remove_file(individual.path)?;
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
