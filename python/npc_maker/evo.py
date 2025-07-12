"""
Evolutionary algorithms and supporting tools.
"""

from os.path import getmtime
from pathlib import Path
import copy
import json
import math
import random
import shlex
import tempfile
import uuid

__all__ = (
    "Genome",
    "Epigenome",
    "Individual",
    "Evolution",
    "Neat",
    "Replayer",
)

class Genome:
    """
    Abstract class for implementing genetic algorithms.
    """
    def parameters(self) -> str:
        """
        Serialize the genome to send it to the control system.
        The message must be a single-line.
        """
        raise TypeError("abstract method called")

    def clone(self) -> 'Genome':
        """
        Create an identical copy of this genome.
        """
        return copy.deepcopy(self)

    def mate(self, other) -> 'Genome':
        """
        Sexually reproduce these two genomes.
        """
        raise TypeError("abstract method called")

    def distance(self, other) -> float:
        """
        Calculate the genetic distance.
        This is used for artificial speciation.
        """
        raise TypeError("abstract method called")

    def save(self) -> object:
        """
        Package the genome into a JSON compatible object.
        """
        raise TypeError("abstract method called")

    @classmethod
    def load(cls, save_data) -> 'Genome':
        """
        Recreate a genome from a saved object.
        """
        raise TypeError("abstract method called")

class Epigenome(Genome):
    """
    Abstract class for implementing genetic algorithms with epigenetic modifications.
    """
    def parameters(self, epigenome) -> str:
        raise TypeError("abstract method called")

    def mate(self, epigenome, other, other_epigenome) -> 'Genome':
        raise TypeError("abstract method called")

class Individual:
    """
    Container for a distinct life-form and all of its associated data.
    """
    def __init__(self, genome, *,
                name=None,
                environment=None,
                population=None,
                controller=None,
                score=None,
                telemetry={},
                epigenome={},
                species=None,
                parents=0,
                children=0,
                birth_date=None,
                death_date=None,
                generation=0,
                ascension=None,
                path=None,
                **extra):
        self.name           = str(name)         if name is not None else str(uuid.uuid4())
        self.environment    = str(environment)  if environment is not None else None
        self.population     = str(population)   if population is not None else None
        self.controller     = _clean_ctrl_command(controller)
        self.genome         = genome
        self.score          = str(score)        if score is not None else None
        self.telemetry      = dict(telemetry)
        self.epigenome      = dict(epigenome)
        self.species        = str(species)      if species is not None else str(uuid.uuid4())
        self.parents        = int(parents)
        self.children       = int(children)
        self.birth_date     = str(birth_date)   if birth_date is not None else None
        self.death_date     = str(death_date)   if death_date is not None else None
        self.generation     = int(generation)
        self.ascension      = int(ascension)    if ascension is not None else None
        self.extra          = extra
        self.path           = Path(path)        if path is not None else None

    def get_name(self) -> str:
        """
        Get this individual's name, which is a UUID string.
        """
        return self.name

    def get_environment(self) -> str:
        """
        Get the name of environment which contains this individual.
        """
        return self.environment

    def get_population(self) -> str:
        """
        Get the name of this individual's population.
        """
        return self.population

    def get_controller(self) -> list:
        """
        Get the command line invocation for the controller program.
        """
        return self.controller

    def get_genome(self) -> Genome:
        """
        Get this individual's genetic data.
        """
        return self.genome

    def get_score(self) -> str:
        """
        Get the most recently assigned score,
        or None if it has not been assigned yet.
        """
        return self.score

    def get_custom_score(self, score_function="score") -> float:
        """
        Apply a custom scoring function to this individual.

        Several classes in this module accept an optional custom score function,
        and they delegate to this method.

        Argument score_function must be one of the following:
            * A callable function: f(individual) -> float,
            * The word "score",
            * The word "ascension",
            * A key in the individual's telemetry dictionary. The corresponding
              value will be converted in to a float.
        """
        if callable(score_function):
            score = score_function(self)
        elif not score_function or score_function == "score":
            score = self.score
        elif score_function == "ascension":
            score = self.ascension
        elif score_function in self.telemetry:
            score = self.telemetry[score_function]
        else:
            raise ValueError("unrecognized score function " + repr(score_function))
        # 
        if score is None:
            score = math.nan
        # 
        return float(score)

    def get_telemetry(self) -> dict:
        """
        Get the environmental info dictionary.

        Returns a reference to the individual's internal "telemetry" dictionary,
        modifications are permanent!
        """
        return self.telemetry

    def get_epigenome(self) -> dict:
        """
        Get the epigenetic info dictionary.

        Returns a reference to the individual's internal "epigenome" dictionary,
        modifications are permanent!
        """
        return self.epigenome

    def get_species(self) -> str:
        """
        Get the species UUID.

        Mating may be restricted to individuals of the same species.
        """
        return self.species

    def get_parents(self) -> int:
        """
        How many parents does this individual have?
        """
        return self.parents

    def get_children(self) -> int:
        """
        How many children does this individual have?
        """
        return self.children

    def get_birth_date(self) -> str:
        """
        The time of birth, as a UTC timestamp,
        or None if this individual has not yet been born.
        """
        return self.birth_date

    def get_death_date(self) -> str:
        """
        The time of death, as a UTC timestamp,
        or None if this individual has not yet died.
        """
        return self.death_date

    def get_generation(self) -> int:
        """
        How many cohorts of the population size passed before this individual was born?
        """
        return self.generation

    def get_ascension(self) -> int:
        """
        How many individuals died before this individual?
        Returns None if this individual has not yet died.
        """
        return self.ascension

    def get_extra(self) -> dict:
        """
        Get any unrecognized fields that were found in the individual's JSON object.

        Returns a reference to this individual's internal data.
        Changes made to the returned value will persist with the individual.
        """
        return self.extra

    def get_path(self) -> Path:
        """
        Returns the file path this individual was loaded from or saved to.
        Returns None if this individual has not touched the file system.
        """
        return self.path

    def get_parameters(self):
        """
        Serialize the genome into a single-line string for the control system.
        """
        if isinstance(self.genome, Epigenome):
            return self.genome.parameters(self.epigenome)
        elif isinstance(self.genome, Genome):
            return self.genome.parameters()
        else:
            return self.genome

    def clone(self):
        """
        Create an identical copy of this genome.
        """
        # Clone the genetic material.
        if isinstance(self.genome, Genome):
            clone_genome = self.genome.clone()
        else:
            clone_genome = copy.deepcopy(self.genome)
        # Make a new individual with the copied genetics.
        clone = Individual(clone_genome,
                epigenome   = self.epigenome,
                environment = self.environment,
                population  = self.population,
                species     = self.species,
                controller  = self.controller,
                generation  = self.generation + 1,
                parents     = 1)
        self.children += 1
        return clone

    def mate(self, other, speciation_distance=None):
        """
        Sexually reproduce these two individuals.
        """
        # Mate the genetic material.
        if isinstance(self.genome, Epigenome):
            child_genome = self.genome.mate(self.epigenome, other.genome, other.epigenome)
        elif isinstance(self.genome, Genome):
            child_genome = self.genome.mate(other.genome)
        else:
            raise TypeError(f"expected npc_maker.evo.Genome, found {type(self.genome)}")
        # Determine which species the child belongs to.
        if speciation_distance is None:
            species = self.species
        else:
            species = None
            for parent in (self, other):
                if parent.genome.distance(child_genome) < speciation_distance:
                    species = parent.species
                    break
        # 
        child = Individual(child_genome,
                environment = self.environment,
                population  = self.population,
                species     = species,
                controller  = self.controller,
                generation  = max(self.generation, other.generation) + 1,
                parents     = (1 if self == other else 2))
        # Update the parent's child count.
        self.children += 1
        if self != other:
            other.children += 1
        return child

    def save(self, path) -> Path:
        """
        Serialize this individual to JSON and write it to a file.

        Argument path is the directory to save in.

        The filename will be either the individual's name or its ascension number,
        and the file extension will be ".json"

        Returns the save file's path.
        """
        if self.name is not None:
            filename = self.name
        elif self.ascension is not None:
            filename = str(self.ascension)
        else:
            raise ValueError("individual has neither name nor ascension")
        path = Path(path)
        assert path.is_dir()
        path = path.joinpath(filename + ".json")
        # Unofficial fields, in case of conflict these take lower precedence.
        data = dict(self.extra)
        # Required fields.
        if isinstance(self.genome, Genome):
            data["genome"]  = self.genome.save()
        else:
            data["genome"]  = self.genome
        data["telemetry"]   = self.telemetry
        data["epigenome"]   = self.epigenome
        data["parents"]     = self.parents
        data["children"]    = self.children
        data["species"]     = self.species
        data["generation"]  = self.generation
        # Optional fields.
        if self.name is not None:        data["name"]        = self.name
        if self.ascension is not None:   data["ascension"]   = self.ascension
        if self.environment is not None: data["environment"] = self.environment
        if self.population is not None:  data["population"]  = self.population
        if self.controller is not None:  data["controller"]  = self.controller
        if self.score is not None:       data["score"]       = self.score
        if self.birth_date is not None:  data["birth_date"]  = self.birth_date
        if self.death_date is not None:  data["death_date"]  = self.death_date
        # Convert paths to strings for JSON serialization.
        if self.controller is not None:
            data["controller"]    = list(data["controller"])
            data["controller"][0] = str(data["controller"][0])
        # 
        self.path = path
        with open(path, 'wt') as file:
            json.dump(data, file)
        return path

    @classmethod
    def load(cls, path, genome_cls) -> 'Individual':
        """
        Load a previously saved individual.
        """
        path = Path(path)
        with open(path, 'rt') as file:
            data = json.load(file)
        # Update the path.
        data["path"] = path
        # Unpack the user's genome.
        if genome is not None:
            data["genome"] = genome_cls.load(data["genome"])
        # 
        return cls(**data)

def _clean_ctrl_command(command):
    if command is None:
        return None
    elif isinstance(command, Path):
        command = [command]
    elif isinstance(command, str):
        command = shlex.split(command)
    else:
        command = list(command)
    # Don't resolve the path yet in case the PWD changes.
    program = Path(command[0]) # .expanduser().resolve()
    command[0] = program
    for index in range(1, len(command)):
        arg = command[index]
        if not isinstance(arg, bytes) and not isinstance(arg, str):
            command[index] = str(arg)
    return command

class Evolution:
    """
    Abstract class for implementing evolutionary algorithms
    and other similar parameter optimization techniques.
    """
    def birth(self, parents=[]) -> Individual:
        """
        Argument parents is a list of Individual objects.

        Return a new Individual object with the "controller" and "genome"
        attributes set. All other attributes are optional.
        The genome may be any JSON-encodable python object.
        """
        raise TypeError("abstract method called")

    def death(self, individual):
        """
        Notification of an individual's death.
        """
        raise TypeError("abstract method called")

class Replayer(API):
    """
    Replay saved individuals
    """
    def __init__(self, path, select="Random", score="score"):
        """
        Argument path is the directory containing the saved individuals.
                 Individuals must have the file extension ".json"

        Argument select is a mate selection algorithm.

        Argument score is an optional custom scoring function.
        """
        self._path          = Path(path).expanduser().resolve()
        self._select        = select
        self._score         = score
        self._population    = [] # List of paths.
        self._scores        = [] # Runs parallel to the population list.
        self._buffer        = [] # Queue of selected individuals wait to be born.

    def path(self):
        return self._path

    def get_population(self):
        """
        Returns a list of file paths.
        """
        self._scan()
        return self._population

    def birth(self, parents=[]):
        self._scan()
        if not self._buffer:
            buffer_size = max(128, len(self._population))
            indices = self._select.select(buffer_size, self._scores)
            self._buffer = [self._population[i] for i in indices]
        path = self._buffer.pop()
        individual = Individual.load(path)
        return [individual.get_genome(), individual.get_info()]

    def death(self, individual):
        pass

    def _scan(self):
        if getattr(self, "_scan_time", -1) == getmtime(self._path):
            return
        content = [p for p in self._path.iterdir() if p.suffix.lower() == ".json"]
        content.sort()
        if content == self._population:
            return
        self._population = content
        self._scan_time = getmtime(self._path)
        self._calc_scores()
        self._buffer.clear()

    def _calc_scores(self):
        self._scores = []
        for path in self._population:
            individual = Individual.load(path)
            score = individual.get_custom_score(self._score)
            self._scores.append(score)

class _Recorder:
    def __init__(self, path, leaderboard, hall_of_fame):
        # Clean and save the arguments.
        if path is None:
            self._tempdir   = tempfile.TemporaryDirectory()
            path            = self._tempdir.name
        self._path          = Path(path)
        self._leaderboard    = int(leaderboard) if leaderboard is not None else 0
        self._hall_of_fame   = int(hall_of_fame) if hall_of_fame is not None else 0
        assert self._path.is_dir()
        assert self._leaderboard >= 0
        assert self._hall_of_fame >= 0
        # 
        if self._leaderboard: self._load_leaderboard()
        if self._hall_of_fame: self._load_hall_of_fame()

    def get_path(self):
        """
        Returns the path argument or a temporary directory.
        """
        return self._path

    def get_leaderboard_path(self):
        """
        Returns a path or None if the leaderboard is disabled.
        """
        if self._leaderboard:
            return self._path.joinpath("leaderboard")
        else:
            return None

    def get_hall_of_fame_path(self):
        """
        Returns a path or None if the hall of fame is disabled.
        """
        if self._hall_of_fame:
            return self._path.joinpath("hall_of_fame")
        else:
            return None

    def _record_death(self, individual, score):
        if self._leaderboard:  self._update_leaderboard(individual, score)
        if self._hall_of_fame: self._update_hall_of_fame(individual, score)

    def _load_leaderboard(self):
        self._leaderboard_data = []
        # 
        leaderboard_path = self.get_leaderboard_path()
        if not leaderboard_path.exists():
            leaderboard_path.mkdir()
        # 
        for path in leaderboard_path.iterdir():
            if path.suffix.lower() == ".json":
                individual  = Individual.load(path)
                score       = individual.get_custom_score(self.score_function)
                ascension   = individual.get_ascension()
                entry       = (score, -ascension, path)
                self._leaderboard_data.append(entry)
        self._settle_leaderboard()

    def _update_leaderboard(self, individual, score):
        # Check if this individual made it onto the leaderboard.
        leaderboard_is_full = len(self._leaderboard_data) >= self._leaderboard
        if leaderboard_is_full and score <= self._leaderboard_data[-1][0]:
            return
        # 
        path = self.get_leaderboard_path()
        individual.save(path)
        ascension   = individual.get_ascension()
        entry       = (score, -ascension, individual.get_path())
        self._leaderboard_data.append(entry)
        # 
        self._settle_leaderboard()

    def _settle_leaderboard(self):
        """ Sort and prune the leaderboard. """
        self._leaderboard_data.sort(reverse=True)
        while len(self._leaderboard_data) > self._leaderboard:
            (score, neg_ascension, path) = self._leaderboard_data.pop()
            path.unlink()

    def get_leaderboard(self):
        """
        The leaderboard is a list of pairs of (path, score).
        It is sorted descending so leaderboard[0] is the best individual.
        """
        return [(path, score) for (score, neg_ascension, path) in self._leaderboard_data]

    def get_best(self):
        """
        Returns the best individual ever.

        Only available if the leaderboard is enabled.
        Returns None if the leaderboard is empty.
        """
        if not self._leaderboard:
            raise ValueError("leaderboard is disabled")
        if not self._leaderboard_data:
            return None
        (score, neg_ascension, path) = max(self._leaderboard_data)
        return Individual.load(path)

    def _load_hall_of_fame(self):
        self._hall_of_fame_data         = []
        self._hall_of_fame_candidates   = []
        # Get the path and make sure that it exists.
        hall_of_fame_path = self.get_hall_of_fame_path()
        if not hall_of_fame_path.exists():
            hall_of_fame_path.mkdir()
        # Load individuals from file.
        for path in hall_of_fame_path.iterdir():
            if path.suffix.lower() == ".json":
                individual = Individual.load(path)
                self._hall_of_fame_data.append(individual)
        # Sort the data chronologically.
        self._hall_of_fame_data.sort(key=lambda x: x.get_ascension())
        # Replace the individuals with their file-paths.
        self._hall_of_fame_data = [x.get_path() for x in self._hall_of_fame_data]

    def _update_hall_of_fame(self, individual, score):
        ascension   = individual.get_ascension()
        entry       = (score, -ascension, individual)
        self._hall_of_fame_candidates.push(entry)

    def _settle_hall_of_fame(self):
        path = self.get_hall_of_fame_path()
        # 
        self._hall_of_fame_candidates.sort()
        winners = self._hall_of_fame_candidates[-self._hall_of_fame:]
        winners = [individual for (score, neg_ascension, individual) in winners]
        winners.sort(key=lambda individual: individual.get_ascension())
        for individual in winners:
            individual.save(path)
            self._hall_of_fame_data.append(individual.get_path())
        # 
        self._hall_of_fame_candidates.clear()

    def get_hall_of_fame(self):
        """
        The hall of fame is a list of paths of the best scoring individuals from
        each generation. It is sorted in chronological order so hall_of_fame[0]
        is the oldest individual.
        """
        return list(self._hall_of_fame_data)

class Neat(Evolution, _Recorder):
    """
    """
    def __init__(self, seed,
            population_size,
            speciation_distance,
            species_distribution,
            mate_selection,
            score_function="score",
            path=None,
            leaderboard=0,
            hall_of_fame=0,):
        """
        Argument seed is the initial individual to begin evolution from.

        Argument population_size is 

        Argument elites is the number of high scoring individuals to be cloned
                 (without modification) into each new generation.

        Argument select is a mate selection algorithm.
                 See the `mate_selection` package for more information.

        Argument score is an optional custom scoring function.

        Argument path is the directory to record data to. This class will
                 incorporate any existing data in the directory to correctly
                 resume recording after a program shutdown.
                 If omitted then this will create a temporary directory.

        Argument leaderboard is the number top performing of individuals to save.
                 If zero or None (the default) then the leaderboard is disabled.
                 Individuals are saved into the directory: path/leaderboard

        Argument hall_of_fame is the number of individuals in each generation /
                 cohort of the hall of fame. The best individual from each cohort
                 will be saved into the hall of fame.
                 If zero or None (the default) then the hall of fame is disabled.
                 Individuals are saved into the directory: path/hall_of_fame
        """
        # Clean and save the arguments.
        assert isinstance(seed, Individual)
        self.population_size        = int(population_size)
        self.speciation_distance    = float(speciation_distance)
        self.species_distribution   = species_distribution
        self.mate_selection         = mate_selection
        self.score_function         = score_function
        assert self.population_size     > 0
        assert self.speciation_distance > 0
        _Recorder.__init__(self, path, leaderboard, hall_of_fame)
        # Initialize our internal data structures.
        self._ascension     = 0 # Number of individuals who have died.
        self._generation    = 0 # Generation counter.
        self._species       = [] # Pairs of (avg-score, members-list), the current mating population.
        self._parents       = [] # Pairs of individuals, buffer of potential mates.
        self._elites        = [] # Exemplars of each species from the previous generation, waiting to be cloned into the next generation.
        self._waiting       = [] # Evaluated individuals, the next generation.
        self._stagnant      = {} # Species stagnation data: pairs of (high-score, generations)
        # Create the first generation by mating the seed with itself.
        if seed.score is None:
            seed.score = 0.0
        self._species.append((seed.score, [seed]))

    def get_ascension(self) -> int:
        """
        Returns the total number of individuals who have died.
        """
        return self._ascension

    def get_generation(self):
        """
        Returns the number of generations that have completely passed.
        """
        return self._generation

    def _score(self, individual):
        return individual.get_custom_score(self.score_function)

    def _rollover(self):
        """
        Discard the old generation and move the next generation into its place.
        """
        # Sort the next generation into its species.
        self._waiting.sort(key=lambda individual: individual.get_species())
        self._species.clear()
        # Scan for contiguous ranges of the same species.
        prev_uuid = self._waiting[0].get_species()
        members = []
        def close_species():
            nonlocal members
            score = sum(self._score(individual) for individual in members) / len(members)
            self._species.append((score, members))
            members = []
        # 
        for individual in self._waiting:
            uuid = individual.get_species()
            # Close out the previous species.
            if uuid != prev_uuid:
                close_species()
                prev_uuid = uuid
            # 
            members.append(individual)
        # Close out the final species.
        close_species()
        # Update the species stagnation info.
        remove_species = []
        for index, (_, members) in enumerate(self._species):
            species = members[0].get_species()
            high_score = max(self._score(individual) for individual in members)
            if species not in self._stagnant or high_score > self._stagnant[species][0]:
                self._stagnant[species] = [high_score, 0]
            else:
                self._stagnant[species][1] += 1
                if self._stagnant[species][1] >= 15:
                    remove_species.append(index)
        for index in reversed(remove_species):
            self._species.pop(index)
        # Clone and reevaluate the best individuals from each species.
        self._elites.clear()
        for (_, members) in self._species:
            if len(members) >= 5:
                best = max(members, key=self._score)
                self._elites.append(best)
        # Reset in preparation for the next generation.
        self._parents.clear()
        self._waiting.clear()
        self._generation += 1
        if self._hall_of_fame: self._settle_hall_of_fame()
        # 
        if not self._species:
            raise RuntimeError("total extinction")

    def _sample(self):
        """
        Refill the _parents buffer.
        """
        # Distribute the offspring to species according to their average score.
        scores = [score for (score, members) in self._species]
        selected = self.species_distribution.select(self.population_size, scores)
        # Count how many offspring were allocated to each species.
        histogram = [0 for _ in range(len(self._species))]
        for x in selected:
            histogram[x] += 1
        # Sample parents from each species.
        for (num_offspring, (_, members)) in zip(histogram, self._species):
            # Cull species when they get too small.
            if num_offspring <= 1:
                continue
            # 
            scores = [self._score(individual) for individual in members]
            for pair in self.mate_selection.pairs(num_offspring, scores):
                self._parents.append([members[index] for index in pair])
        # 
        random.shuffle(self._parents)

    def birth(self, parents=[]) -> 'Individual':
        # 
        if len(self._waiting) >= self.population_size:
            self._rollover()
        # 
        if not self._parents and not self._elites:
            self._sample()
        # 
        if self._elites:
            return self._elites.pop().clone()

        elif random.random() < 0.25:
            parent, _ = self._parents.pop()
            return parent.mate(parent, self.speciation_distance)

        else:
            mother, father = self._parents.pop()
            if self._score(father) > self._score(mother):
                mother, father = father, mother
            return mother.mate(father, self.speciation_distance)

    def death(self, individual):
        if individual is None:
            return
        assert isinstance(individual, Individual)
        # Replace the individual's name with its ascension number.
        individual.name      = None
        individual.ascension = self._ascension
        self._ascension += 1
        # Ignore individuals who die without a valid score.
        score = individual.get_custom_score(self.score_function)
        if score is None or math.isnan(score) or score == -math.inf:
            return
        # 
        self._waiting.append(individual)
        self._record_death(individual, score)
