"""
Evolutionary algorithms and supporting tools.
"""

from pathlib import Path
import copy
import io
import json
import math
import os
import os.path
import pickle
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
    def parameters(self) -> bytes:
        """
        Package the genome for sending it to the control system.
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

    def save(self) -> bytes:
        """
        Serialize the genome into a binary object for long-term storage.
        """
        return pickle.dumps(self)

    @classmethod
    def load(cls, save_data) -> 'Genome':
        """
        Recreate a genome from a saved object.
        """
        return pickle.loads(save_data)

class Epigenome(Genome):
    """
    Abstract class for implementing genetic algorithms with epigenetic modifications.
    """
    def parameters(self, epigenome) -> bytes:
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
                parents=[],
                children=[],
                birth_date=None,
                death_date=None,
                generation=0,
                ascension=None,
                path=None,
                **extra):
        self.name           = str(name)         if name is not None else str(uuid.uuid4())
        self.environment    = str(environment)  if environment is not None else None
        self.population     = str(population)   if population is not None else None
        self.controller     = self._clean_ctrl_command(controller)
        self._genome        = genome
        self._genome_cls    = type(genome)
        self.score          = str(score)        if score is not None else None
        self.telemetry      = dict(telemetry)
        self.epigenome      = dict(epigenome)
        self.species        = str(species)      if species is not None else str(uuid.uuid4())
        self.parents        = [str(name) for name in parents]
        self.children       = [str(name) for name in children]
        self.birth_date     = str(birth_date)   if birth_date is not None else None
        self.death_date     = str(death_date)   if death_date is not None else None
        self.generation     = int(generation)
        self.ascension      = int(ascension)    if ascension is not None else None
        self.extra          = extra
        self.path           = Path(path)        if path is not None else None
        assert isinstance(genome, Genome) or isinstance(genome, bytes) or genome is None
        assert genome is not None or self.path is not None

    @staticmethod
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
        return copy.copy(self.controller)

    def get_genome(self) -> Genome:
        """
        Get this individual's genetic data.
        Genome's are considered immutable.
        """
        if self._genome is None:
            self._load_genome()
        return self._genome

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

    def get_parents(self) -> [str]:
        """
        Get the names of this individual's parents.
        """
        return list(self.parents)

    def get_children(self) -> [str]:
        """
        Get the names of this individual's children.
        """
        return list(self.children)

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
        Format the genome into a binary blob for the control system.
        """
        genome = self.get_genome()
        if isinstance(genome, Epigenome):
            parameters = genome.parameters(self.epigenome)
        elif isinstance(genome, Genome):
            parameters = genome.parameters()
        else:
            parameters = genome
        # Check data type.
        if isinstance(parameters, str):
            parameters = parameters.encode("utf-8")
        assert isinstance(parameters, bytes)
        return parameters

    def birth(self) -> (dict, bytes):
        """
        Package up this individual for sending it to an environment.
        """
        population = self.population
        if population is None:
            population = ""
        controller = self.get_controller()
        if not controller:
            raise ValueError("missing controller")
        controller[0] = str(controller[0]) # Convert Path to String
        parameters = self.get_parameters()
        metadata = {
            "name": self.name,
            "population": population,
            "parents": self.parents,
            "controller": controller,
            "genome": len(parameters),
        }
        return (metadata, parameters)

    def clone(self):
        """
        Create an identical copy of this genome.
        """
        # Clone the genetic material.
        genome = self.get_genome()
        if isinstance(genome, Genome):
            clone_genome = genome.clone()
        else:
            clone_genome = copy.deepcopy(genome)
        # Make a new individual with the copied genetics.
        clone = Individual(clone_genome,
                epigenome   = self.epigenome,
                environment = self.environment,
                population  = self.population,
                species     = self.species,
                controller  = self.controller,
                generation  = self.generation + 1,
                parents     = [self.name])
        self.children.append(clone.name)
        return clone

    def mate(self, other, speciation_distance=None):
        """
        Sexually reproduce these two individuals.
        """
        # Mate the genetic material.
        self_genome = self.get_genome()
        if isinstance(self_genome, Epigenome):
            child_genome = self_genome.mate(self.epigenome, other.get_genome(), other.epigenome)
        elif isinstance(self_genome, Genome):
            child_genome = self_genome.mate(other.get_genome())
        else:
            raise TypeError(f"expected npc_maker.evo.Genome, found {type(self_genome)}")
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
                parents     = [self.name, other.name])
        # Update the parent's child count.
        self.children.append(child.name)
        if self != other:
            other.children.append(child.name)
        return child

    def save(self, path) -> Path:
        """
        Serialize this individual to JSON and write it to a file.

        Argument path is the directory to save in.

        The filename will be either the individual's name or its ascension number,
        and the file extension will be ".json"

        Returns the save file's path.
        """
        if self.ascension is not None:
            filename = str(self.ascension)
        elif self.name is not None:
            filename = self.name
        else:
            raise ValueError("individual has neither name nor ascension")
        path = Path(path)
        assert path.is_dir()
        path = path.joinpath(filename + ".json")
        # 
        genome = self.get_genome()
        if isinstance(genome, Genome):
            genome = genome.save()
        assert isinstance(genome, bytes)
        # Unofficial fields, in case of conflict these take lower precedence.
        data = dict(self.extra)
        # Required fields.
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
        data = json.dumps(data)
        # Save to a hidden file, sync, and atomic move into place.
        fd, tmp_path = tempfile.mkstemp()
        file = os.fdopen(fd, "wb")
        file.write(data.encode("utf-8"))
        file.write(b'\x00')
        file.write(genome)
        file.flush()
        file.close()
        Path(tmp_path).rename(path)
        # 
        self.path = path
        return path

    @classmethod
    def load(cls, genome_cls, path) -> 'Individual':
        """
        Load a previously saved individual.
        """
        path = Path(path)
        text = b''
        with open(path, 'rb') as file:
            while True:
                chunk = file.read(io.DEFAULT_BUFFER_SIZE)
                split = chunk.split(b'\x00', maxsplit=1)
                text += split[0]
                if len(split) > 1:
                    break
        metadata = json.loads(text)
        metadata["path"] = path
        self = cls(None, **metadata)
        self._genome_cls = genome_cls
        return self

    def _load_genome(self):
        with open(self.get_path(), 'rb') as file:
            data = file.read()
        text, binary = data.split(b'\x00', maxsplit=1)
        self._genome = self._genome_cls.load(binary)

class Evolution:
    """
    Abstract class for implementing evolutionary algorithms
    and other similar parameter optimization techniques.
    """
    def spawn(self):
        """
        Returns either a single parent to clone or a pair of parents to mate together.
        """
        raise TypeError("abstract method called")

    def death(self, individual):
        """
        Notification of an individual's death.
        """
        raise TypeError("abstract method called")

class Replayer(Evolution):
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
        self._scan_time     = -1
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
        return list(self._population)

    def spawn(self):
        self._scan()
        if not self._buffer:
            buffer_size = max(128, len(self._population))
            indices = self._select.select(buffer_size, self._scores)
            self._buffer.extend(self._population[i] for i in indices)
        return (self._buffer.pop(),)

    def death(self, individual):
        pass

    def _scan(self):
        if self._scan_time == os.path.getmtime(self._path):
            return
        content = [p for p in self._path.iterdir() if p.suffix.lower() == ".json"]
        content.sort()
        if content == self._population:
            return
        self._population = content
        self._scan_time = os.path.getmtime(self._path)
        self._calc_scores()
        self._buffer.clear()

    def _calc_scores(self):
        self._scores.clear()
        for path in self._population:
            individual = Individual.load(path)
            score = individual.get_custom_score(self._score)
            self._scores.append(score)

class Neat(Evolution):
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
        assert seed.get_controller()
        if seed.score is None: seed.score = 0.0
        self.genome_cls             = seed._genome_cls
        self.population_size        = int(population_size)
        self.speciation_distance    = float(speciation_distance)
        self.species_distribution   = species_distribution
        self.mate_selection         = mate_selection
        self.score_function         = score_function
        assert self.population_size     > 0
        assert self.speciation_distance > 0
        # Setup file system.
        if path is None:
            self._tempdir   = tempfile.TemporaryDirectory() # Keep alive for the lifetime of this object.
            path            = self._tempdir.name
        self._path          = Path(path)
        if not self._path.exists():
            self._path.mkdir()
        assert self._path.is_dir()
        # Setup data recording.
        self._leaderboard   = int(leaderboard) if leaderboard is not None else 0
        self._hall_of_fame  = int(hall_of_fame) if hall_of_fame is not None else 0
        assert self._leaderboard >= 0
        assert self._hall_of_fame >= 0
        if self._leaderboard: self._load_leaderboard()
        if self._hall_of_fame: self._load_hall_of_fame()
        # Setup our internal data structures.
        self._ascension = 0 # Number of individuals who have died.
        self._parents   = [] # Pairs of individuals, buffer of potential mates.
        self._load_generation(seed)
        self._load_population()

    def _load_generation(self, seed):
        generations = []
        for path in self.get_path().iterdir():
            if path.is_dir():
                try:
                    gen = int(path.name)
                except ValueError:
                    continue
                assert gen >= 0
                generations.append(gen)
        if not generations:
            self._generation = 0
            self._next_generation_size = 0
            self._get_generation_path(0).mkdir()
            self._get_generation_path(1).mkdir()
            seed.save(self._get_generation_path(0))
        else:
            next_generation = max(generations)
            current_generation = next_generation - 1
            assert current_generation in generations
            self._generation = current_generation
            self._next_generation_size = len(list(self._get_generation_path(next_generation).iterdir()))

    def _load_population(self):
        self._population = [] # Individuals who are currently eligable to mate.
        self._species    = {} # Species UUID -> (avg-score, members-list).
        # Load all individuals and sort them by species.
        for path in self._get_generation_path(self._generation).iterdir():
            individual = Individual.load(self.genome_cls, path)
            self._population.append(individual)
            self._species.setdefault(individual.get_species(), []).append(individual)
        # Calculate each species' average score.
        for uuid, members in self._species.items():
            score = sum(self._score(individual) for individual in members) / len(members)
            self._species[uuid] = (score, members)

    def _load_leaderboard(self):
        leaderboard_path = self.get_leaderboard_path()
        if not leaderboard_path.exists():
            leaderboard_path.mkdir()
        # 
        self._leaderboard_data = []
        for path in leaderboard_path.iterdir():
            self._leaderboard_data.append(Individual.load(self.genome_cls, path))
        self._leaderboard_data.sort(reverse=True, key=lambda x: (self._score(x), -x.ascension))

    def _load_hall_of_fame(self):
        self._hall_of_fame_data = []
        # Get the path and make sure that it exists.
        hall_of_fame_path = self.get_hall_of_fame_path()
        if not hall_of_fame_path.exists():
            hall_of_fame_path.mkdir()
        # Load individuals from file.
        for path in hall_of_fame_path.iterdir():
            individual = Individual.load(self.genome_cls, path)
            self._hall_of_fame_data.append(individual)
        # Sort the data chronologically.
        self._hall_of_fame_data.sort(key=lambda x: x.get_ascension())

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

    def get_path(self):
        """
        Returns the path argument or a temporary directory.
        """
        return self._path

    def _get_generation_path(self, generation):
        """
        Each generation is stored in its own directory.
        """
        return self._path.joinpath(str(generation))

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

    def get_leaderboard(self):
        """
        The leaderboard is sorted descending so leaderboard[0] is the best individual.
        """
        if self._leaderboard:
            return list(self._leaderboard_data)
        else:
            return None

    def get_hall_of_fame(self):
        """
        The hall of fame is a list the best scoring individuals from each generation.
        It is sorted in chronological order so hall_of_fame[0] is the oldest individual.
        """
        if self._hall_of_fame:
            return list(self._hall_of_fame_data)
        else:
            return None

    def get_best(self):
        """
        Returns the best individual ever.

        Only available if the leaderboard is enabled.
        Returns None if the leaderboard is empty.
        """
        if not self._leaderboard:
            raise ValueError("leaderboard is disabled")
        elif not self._leaderboard_data:
            return None
        else:
            return self._leaderboard_data[0]

    def _score(self, individual):
        return individual.get_custom_score(self.score_function)

    def spawn(self) -> (Path, Path):
        if self._next_generation_size >= self.population_size:
            self._rollover()
        if not self._parents:
            self._sample()
        mother, father = self._parents.pop()
        if self._score(mother) < self._score(father):
            mother, father = father, mother
        return mother, father

    def _rollover(self):
        """
        Discard the old generation and move the next generation into its place.
        """
        self._generation += 1
        self._get_generation_path(self._generation + 1).mkdir()
        self._next_generation_size = 0
        self._parents.clear()
        self._load_population()
        if self._hall_of_fame: self._rollover_hall_of_fame()
        if self._leaderboard: self._rollover_leaderboard()
        # Discard a previous generation.
        prev_prev_generation = self._get_generation_path(self._generation - 2)
        if prev_prev_generation.exists():
            for path in prev_prev_generation.iterdir():
                path.unlink()
            prev_prev_generation.rmdir()

    def _rollover_leaderboard(self):
        in_leaderboard = lambda path: path and path.is_relative_to(leaderboard_path)
        # Add all contestants to the leaderboard.
        self._leaderboard_data.extend(self._population)
        self._leaderboard_data.sort(reverse=True, key=lambda x: (self._score(x), -x.ascension))
        # Discard low performing individuals.
        leaderboard_path = self.get_leaderboard_path()
        while len(self._leaderboard_data) > self._leaderboard:
            individual = self._leaderboard_data.pop()
            if in_leaderboard(individual.path):
                individual.path.unlink()
        # Ensure all remaining individuals are saved to the leaderboard directory.
        for individual in self._leaderboard_data:
            if not in_leaderboard(individual.path):
                individual.save(leaderboard_path)

    def _rollover_hall_of_fame(self):
        hall_of_fame_path = self.get_hall_of_fame_path()
        self._population.sort(key=lambda individual: (self._score(individual), -individual.get_ascension()))
        winners = population[-self._hall_of_fame:]
        winners.sort(key=lambda individual: individual.get_ascension())
        for individual in winners:
            individual.save(hall_of_fame_path)
            self._hall_of_fame_data.append(individual)

    def _sample(self):
        """
        Refill the _parents buffer.
        """
        # Distribute the offspring to species according to their average score.
        scores = [score for (score, members) in self._species.values()]
        selected = self.species_distribution.select(self.population_size, scores)
        # Count how many offspring were allocated to each species.
        histogram = [0 for _ in range(len(self._species))]
        for x in selected:
            histogram[x] += 1
        # Sample parents from each species.
        for (num_offspring, (_, members)) in zip(histogram, self._species.values()):
            scores = [self._score(individual) for individual in members]
            for pair in self.mate_selection.pairs(num_offspring, scores):
                self._parents.append([members[index] for index in pair])
        # 
        random.shuffle(self._parents)

    def death(self, individual):
        # Validate the input.
        if individual is None:
            return
        if isinstance(individual, str) or isinstance(individual, Path):
            individual = Individual.load(self.genome_cls, individual)
        assert isinstance(individual, Individual)
        assert individual._genome_cls is self.genome_cls
        # 
        individual.ascension = self._ascension
        self._ascension += 1
        # Ignore individuals who die without a valid score.
        score = self._score(individual)
        if score is None or math.isnan(score) or score == -math.inf:
            return
        # Stash the individual.
        individual.save(self._get_generation_path(self._generation + 1))
        self._next_generation_size += 1
