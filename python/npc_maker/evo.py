"""
Evolutionary algorithms and supporting tools.
"""

from os.path import getmtime
from pathlib import Path
import json
import math
import random
import shlex
import tempfile
import uuid

__all__ = (
    "API",
    "Genome",
    "Individual",
    "Evolution",
    "Replayer",
)

class API:
    """
    Abstract class for implementing evolutionary algorithms
    and other similar parameter optimization techniques.
    """
    def birth(self, parents=[]) -> 'Individual':
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

class Genome:
    """
    Abstract class for implementing genetic algorithms.
    """
    def parameters(self) -> str:
        """
        Serialize the genome into a single-line string for the control system.
        """
        raise TypeError("abstract method called")

    def mate(self, other) -> 'Genome':
        """
        Called when two individuals sexually reproduce.
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
                info={},
                species=None,
                parents=None,
                children=None,
                birth_date=None,
                death_date=None,
                generation=None,
                ascension=None,
                **extras):
        assert isinstance(genome, Genome)
        self.name           = str(name) if name is not None else str(uuid.uuid4())
        self.environment    = str(environment) if environment is not None else None
        self.population     = str(population) if population is not None else None
        self.controller     = _clean_ctrl_command(controller)
        self.genome         = genome
        self.score          = score
        self.info           = dict(info)
        self.species        = str(species) if species is not None else None
        self.parents        = parents
        self.children       = children
        self.birth_date     = birth_date
        self.death_date     = death_date
        self.generation     = generation
        self.ascension      = ascension
        self.extras         = extras
        self.path           = None

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

    def get_name(self) -> str:
        """
        Get this individual's name, which is a UUID string.

        Note: individual's lose their name when they die.
        """
        return self.name

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

    def get_custom_score(self, score_function) -> float:
        """
        Apply a custom scoring function to this individual.

        Several classes in this module accept an optional custom score function,
        and they delegate to this method.

        Argument score_function must be one of the following:
            * A callable function: f(individual) -> float,
            * The word "score",
            * The word "ascension",
            * A key in the individual's info dictionary. The corresponding value
              will be converted in to a float.
        """
        if callable(score_function):
            score = score_function(self)
        elif not score_function or score_function == "score":
            score = self.score
        elif score_function == "ascension":
            if self.ascension is None:
                score = math.nan
            else:
                score = self.ascension
        elif score_function in self.info:
            score = self.info[score_function]
        else:
            raise ValueError("unrecognized score function " + repr(score_function))
        return float(score)

    def get_info(self) -> dict:
        """
        Get the current info.

        Note: this returns a reference to the individual's internal info dict.
        Modifications will become a permanent part of the individual's info.
        """
        return self.info

    def get_species(self) -> str:
        """
        Get the species UUID.

        This is assigned by the NEAT algorithm.
        Mating is restricted to individuals with the same species.
        """
        return self.species

    def get_parents(self) -> int:
        """
        How many parents does this individual have?

        Individuals created by "New" requests have zero parents.
        Individuals created by "Mate" requests have one or more parents.
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

    def get_extras(self) -> dict:
        """
        Get any unrecognized fields that were found in the individual's JSON object.

        Returns a reference to this individual's internal data.
        Changes made to the returned value will persist with the individual.
        """
        return self.extras

    def get_path(self) -> Path:
        """
        Returns the file path this individual was loaded from or saved to.
        Returns None if this individual has not touched the file system.
        """
        return self.path

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
        # Unofficial fields, in case of conflict these take lowest precedence.
        data = dict(self.extras)
        # Required fields.
        data["genome"] = self.genome.save()
        # Optional fields.
        if self.ascension is not None:   data["ascension"]   = self.ascension
        if self.birth_date is not None:  data["birth_date"]  = self.birth_date
        if self.children is not None:    data["children"]    = self.children
        if self.controller is not None:  data["controller"]  = list(self.controller)
        if self.death_date is not None:  data["death_date"]  = self.death_date
        if self.environment is not None: data["environment"] = self.environment
        if self.info is not None:        data["info"]        = self.info
        if self.name is not None:        data["name"]        = self.name
        if self.parents is not None:     data["parents"]     = self.parents
        if self.population is not None:  data["population"]  = self.population
        if self.score is not None:       data["score"]       = self.score
        # Convert paths to strings for JSON serialization.
        if self.controller is not None:
            data["controller"][0] = str(data["controller"][0])
        # 
        self.path = path
        with open(path, 'wt') as file:
            json.dump(data, file)
        return path

    @classmethod
    def load(cls, genome_cls, path, **individual_attributes) -> 'Individual':
        """
        Load a previously saved individual.
        """
        path = Path(path)
        with open(path, 'rt') as file:
            return json.load(file)
        # 
        genome = genome_cls(data.pop("genome"))
        individual = cls.load(genome, **individual_attributes)
        individual.path = path
        # Keyword arguments preempt the saved data.
        for field in individual_attributes:
            data.pop(field, None)
        # Load optional fields.
        individual.ascension   = data.pop("ascension",   individual.ascension)
        individual.birth_date  = data.pop("birth_date",  individual.birth_date)
        individual.children    = data.pop("children",    individual.children)
        individual.controller  = data.pop("controller",  individual.controller)
        individual.death_date  = data.pop("death_date",  individual.death_date)
        individual.environment = data.pop("environment", individual.environment)
        individual.info        = data.pop("info",        individual.info)
        individual.name        = data.pop("name",        individual.name)
        individual.parents     = data.pop("parents",     individual.parents)
        individual.population  = data.pop("population",  individual.population)
        individual.score       = data.pop("score",       individual.score)
        # Convert controller program from string to path.
        individual.controller = _clean_ctrl_command(individual.controller)
        # Preserve any unrecognized fields in case the user wants them later.
        individual.extras.update(data)
        return individual

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

class Evolution(API, _Recorder):
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
        self.individual_class       = type(seed)
        self.population_size        = int(population_size)
        self.speciation_distance    = float(speciation_distance)
        self.species_distribution   = species_distribution
        self.mate_selection         = mate_selection
        self.score_function         = score_function
        assert isinstance(self.individual_class, Individual)
        assert self.population_size     > 0
        assert self.speciation_distance > 0
        _Recorder.__init__(self, path, leaderboard, hall_of_fame)
        # Initialize our internal data structures.
        self._ascension     = 0 # Number of individuals who have died.
        self._generation    = 0 # Generation counter.
        self._species       = [] # Pairs of (avg-score, members-list), the current mating population.
        self._parents       = [] # Pairs of individuals, buffer of potential mates.
        self._waiting       = [] # Evaluated individuals, the next generation.
        self._seed_species(seed)

    def _seed_species(self, seed):
        # Create the first generation by mating the seed with itself.
        species_uuid = str(uuid.uuid4())
        species_members = []
        for _ in range(self.population_size):
            genome = seed.mate(seed)
            individual = self.individual_class(genome,
                    species=species_uuid,
                    score=0.0,
                    parents=2,
                    generation=self.get_generation())
            species_members.append(individual)
        self._species.append((0.0, species_members))

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

    def _rollover(self):
        """
        Discard the old generation and move the next generation into its place.
        """
        # Sort the next generation into its species.
        self._waiting.sort(key=lambda x: x.get_species())
        # Scan for contiguous ranges of the same species.
        self._species.clear()
        prev_uuid = None
        score = 0 # Accumulator for calculating the average score.
        members = []
        for individual in self._waiting:
            uuid = individual.get_species()
            # Close out the previous species.
            if uuid != prev_uuid:
                self._species.push((score / len(members), members))
                # Start the next span of species
                members = []
                score = 0
                prev_uuid = uuid
            # 
            members.append(individual)
            score += individual.get_custom_score(self.score_function)
        # Close out the final species.
        self._species.push((score / len(members), members))
        # Reset in preparation for the next generation.
        self._parents.clear()
        self._waiting.clear()
        self._generation += 1
        if self._hall_of_fame: self._settle_hall_of_fame()

    def _sample(self):
        """
        Refill the _parents buffer.
        """
        if self._parents:
            return
        # Distribute the offspring to species according to their average score.
        scores = [score for (score, members) in self._species]
        selected = self.species_distribution.select(self.population_size, scores)
        # Count how many offspring were allocated to each species.
        histogram = [0 for _ in range(len(self._species))]
        for x in selected:
            histogram[x] += 1
        # Sample parents from each species.
        for (num_offspring, (score, members)) in zip(histogram, self._species):
            scores = [individual.get_score() for individual in members]
            for pair in self.mate_selection.pairs(num_offspring, scores):
                self._parents.append([members[index] for index in pair])
        # 
        random.shuffle(self._parents)

    def birth(self, parents=[]) -> 'Individual':
        # 
        if len(self._waiting) >= self.population_size:
            self._rollover()
        # 
        if not self._parents:
            self._sample()
        # Get the parents and mate them together.
        parents = self._parents.pop()
        child = parents[0].mate(parents[1])
        # 
        if not isinstance(child, Individual):
            child = self.individual_class(genome, parents=2, generation=self.get_generation())
        # Determine which species the child belongs to.
        for parent in parents:
            if parent.distance(child) < self.speciation_distance:
                child.species = parent.species
                break
        else:
            child.species = str(uuid.uuid4())
        # Update the parent's child count.
        for parent in parents:
            parent.children += 1
        # 
        return child

    def death(self, individual):
        if individual is None:
            return
        assert isinstance(individual, self.individual_class)
        # Replace the individual's name with its ascension number.
        individual.name = None
        individual.ascension = self._ascension
        self._ascension += 1
        # Ignore individuals who die without a valid score.
        score = individual.get_custom_score(self.score_function)
        if score is None or math.isnan(score) or score == -math.inf:
            return
        # 
        self._waiting.append(individual)
        _Recorder._record_death(individual, score)
