"""
Evolution API, for making and using evolution services.
"""

from os.path import getmtime
from pathlib import Path
import collections
import copy
import heapq
import json
import random
import math
import shlex
import tempfile
import uuid

__all__ = (
    "Individual",
    "API",
    "Recorder",
    "Replayer",
    "Evolution",
)

def _clean_ctrl_command(command):
    if command is None:
        return None
    elif isinstance(command, Path):
        command = [command]
    elif isinstance(command, str):
        command = shlex.split(command)
    else:
        command = list(command)
    program = Path(command[0]).expanduser().resolve()
    command[0] = program
    for index in range(1, len(command)):
        arg = command[index]
        if not isinstance(arg, bytes) and not isinstance(arg, str):
            command[index] = str(arg)
    return command

class Individual:
    """
    Container for a distinct life-form and all of its associated data.
    """
    def __init__(self, genome, *,
                environment=None,
                population=None,
                controller=None,
                score=None,
                info={},
                parents=None,
                children=None,
                birth_date=None,
                death_date=None,
                ascension=None,
                **extras):
        self.name           = str(uuid.uuid4())
        self.environment    = str(environment) if environment is not None else None
        self.population     = str(population) if population is not None else None
        self.controller     = _clean_ctrl_command(controller)
        self.genome         = genome
        self.score          = score
        self.info           = dict(info)
        self.parents        = parents
        self.children       = children
        self.birth_date     = birth_date
        self.death_date     = death_date
        self.ascension      = ascension
        self.extras         = {}
        self.path           = None

    def get_environment(self):
        """
        Get the name of environment which contains this individual.
        """
        return self.environment

    def get_population(self):
        """
        Get the name of this individual's population.
        """
        return self.population

    def get_name(self):
        """
        Get this individual's name, which is a UUID string.

        Note: individual's lose their name when they die.
        """
        return self.name

    def get_controller(self):
        """
        Get the command line invocation for the controller program.
        """
        return self.controller

    def get_genome(self):
        """
        Get this individual's genetic data.
        The genome may be any JSON encodable object.

        Returns a bundle of decoded JSON data (a python object).
        """
        return self.genome

    def get_score(self):
        """
        Get the most recently assigned score,
        or None if it has not been assigned yet.
        """
        return self.score

    def get_custom_score(self, score_function):
        """
        Apply a custom scoring function to this individual.

        Several classes in this module accept an optional custom score function,
        and they accept anything which this method accepts.

        Argument score_function must be one of the following:
            * A callable function: f(individual) -> float,
            * The word "score",
            * The word "ascension",
            * A key in the individual's info dictionary. The corresponding value
              will be converted in to a float.
        """
        if callable(score_function):
            return score_function(self)
        elif not score_function or score_function == "score":
            return self.score
        elif score_function == "ascension":
            if self.ascension is None:
                return math.nan
            else:
                return self.ascension
        elif score_function in self.info:
            return self.info[score_function]
        else:
            raise ValueError("unrecognized score function " + repr(score_function))

    def get_info(self):
        """
        Get the current info.

        Note: this returns a reference to the individual's internal info dict.
        Modifications will become a permanent part of the individual's info.
        """
        return self.info

    def get_parents(self):
        """
        How many parents does this individual have?

        Individuals created by "New" requests have zero parents.
        Individuals created by "Mate" requests have one or more parents.
        """
        return self.parents

    def get_children(self):
        """
        How many children does this individual have?
        """
        return self.children

    def get_birth_date(self):
        """
        The time of birth, as a UTC timestamp,
        or None if this individual has not yet been born.
        """
        return self.birth_date

    def get_death_date(self):
        """
        The time of death, as a UTC timestamp,
        or None if this individual has not yet died.
        """
        return self.death_date

    def get_ascension(self):
        """
        How many individuals died before this individual?
        Returns None if this individual has not yet died.

        The attribute "individual.ascension" is set by the evolution service.
        Custom evolution services are encouraged to assign ascension numbers.
        """
        return self.ascension

    def get_extras(self):
        """
        Get any unrecognized fields that were found in the individual's JSON object.

        Returns a reference to this individual's internal data.
        Changes made to the returned value will persist with the individual.
        """
        return self.extras

    def get_path(self) -> 'Path':
        """
        Returns the file path this individual was loaded from or saved to.
        Returns None if this individual has not touched the file system.
        """
        return self.path

    def save(self, path):
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
        path = path.joinpath(filename + ".json")
        # Required fields.
        data = {"genome": self.genome}
        # Optional fields.
        if self.ascension is not None:   data["ascension"]   = self.ascension
        if self.birth_date is not None:  data["birth_date"]  = self.birth_date
        if self.children is not None:    data["children"]    = self.children
        if self.controller is not None:  data["controller"]  = self.controller
        if self.death_date is not None:  data["death_date"]  = self.death_date
        if self.environment is not None: data["environment"] = self.environment
        if self.info is not None:        data["info"]        = self.info
        if self.name is not None:        data["name"]        = self.name
        if self.parents is not None:     data["parents"]     = self.parents
        if self.population is not None:  data["population"]  = self.population
        if self.score is not None:       data["score"]       = self.score
        # Unofficial fields.
        data.update(self.extras)
        # Convert paths to strings for JSON serialization.
        def dump_path(obj):
            if isinstance(obj, Path):
                return str(obj)
            else:
                raise TypeError
        # 
        with open(path, 'wt') as file:
            json.dump(data, file, default=dump_path)
        self.path = path
        return path

    def load(path, **kwargs):
        """
        Load a previously saved individual.
        """
        path = Path(path)
        with open(path, 'rt') as file:
            data = json.load(file)
        # 
        individual = Individual(data.pop("genome"), **kwargs)
        individual.path = path
        # Keyword arguments preempt the saved data.
        for field in kwargs:
            data.pop(field)
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
        if individual.controller is not None:
            individual.controller[0] = Path(individual.controller[0])
        # Preserve any unrecognized fields in case the user wants them later.
        individual.extras      = data
        return individual

class API:
    """
    Abstract class for implementing evolutionary algorithms
    and other parameter optimization techniques.

    Users should inherit from this class and implement its methods.
    Then pass an instance of the class to an environment.
    """
    def birth(self, parents) -> 'Individual':
        """
        Abstract Method

        Argument parents is a list of Individual objects.

        Return a new Individual object with the "controller" and "genome"
        attributes set. All other attributes are optional.
        The genome may be any JSON-encodable python object.
        """
        raise TypeError("abstract method called")

    def death(self, individual):
        """
        Abstract Method

        Notification of an individual's death.
        """
        raise TypeError("abstract method called")

class Recorder(API):
    """
    This wrapper records recording high scoring individuals and statistics from
    another evolution API instance
    """
    def __init__(self, service, path=None, leaderboard=None,
                 score="score", filters={}):
        """
        Argument service is the underlying evolution API instance to record from.

        Argument path is the directory to record data to. This class will
                 incorporate any existing data in the directory to correctly
                 resume recording after a program shutdown.
                 If omitted then this will create a temporary directory.

        Argument leaderboard is the number top performing of individuals to save.
                 If zero or None (the default) then the leaderboard is disabled.
                 Individuals are saved into the directory: path/leaderboard

        Argument score is an optional custom scoring function.

        Argument filters is a dictionary of custom filter functions for selecting
                 which individuals to save. Each key-value pair defines a new filter.
                 * The key is the name of the directory to save to.
                 * The value is a callable function: f(individual) -> bool,
                   where returning True will save the individual, False will reject it.
        """
        """
        Argument hall_of_fame is the number of individuals in each generation /
                 cohort of the hall of fame. The best individual from each cohort
                 will be saved into the hall of fame.
                 If zero or None (the default) then the hall of fame is disabled.
                 Individuals are saved into the directory: path/hall_of_fame

        Argument statistics is a dictionary of custom metrics to measure across
                 the population. Each key-value pair defines a new metric.
                 * The key is the file name to save the data to.
                 * The value is a callable function: f(individual) -> float
                 This computes the following statistics for each generation:
                 minimum, maximum, median, mean, and standard deviation.

        Argument histograms is a dictionary of custom metrics to record histograms of.
                 Each key-value pair defines a new metric.
                 * The key is the file name to save the data to.
                 * The value is a pairs of (metric, bins)
                    + Where metric is a callable function: f(individual) -> float,
                    + Where bins is the number of histogram bins to use.
        """
        if path is None:
            self._tempdir   = tempfile.TemporaryDirectory()
            path            = self._tempdir.name
        self.service        = service
        self._path          = Path(path)
        self.leaderboard    = int(leaderboard) if leaderboard is not None else 0
        # self.hall_of_fame   = int(hall_of_fame) if hall_of_fame is not None else 0
        self.score          = score
        self.filters        = dict(filters)
        # self.statistics     = dict(statistics)
        # self.histograms     = dict(histograms)

        assert isinstance(service, API)
        assert self._path.is_dir()
        assert self.leaderboard >= 0
        # assert self.hall_of_fame >= 0

        if self.leaderboard: self._load_leaderboard()
        # if self.hall_of_fame: self._load_hall_of_fame()

    def get_path(self):
        return self._path

    def get_leaderboard_path(self):
        return self._path.joinpath("leaderboard")

    # def get_hall_of_fame_path(self):
    #     return self._path.joinpath("hall_of_fame")

    def birth(self, parents):
        """"""
        return self.service.birth(parents)

    def death(self, individual):
        """"""
        self.service.death(individual)

        score = individual.get_custom_score(self.score)

        if self.leaderboard:  self._update_leaderboard(individual, score)
        # if self.hall_of_fame: self._update_hall_of_fame(individual, score)

        for filter_name, filter_function in self.filters.items():
            if filter_function(individual):
                individual.save(self._path.joinpath(filter_name))

        # for statistic_name, statistic_data in self.statistics.items():
        #     1/0
        # for histogram_name, histogram_data in self.histograms.items():
        #     1/0

    _LeaderEntryType = collections.namedtuple("_LeaderEntry", ("score", "neg_asc"))

    def _LeaderEntry(self, individual):
        score     = individual.get_custom_score(self.score)
        ascension = individual.get_ascension()
        return self._LeaderEntryType(score, -ascension)

    def _load_leaderboard(self):
        self._leaderboard_data = []
        leaderboard_path = self.get_leaderboard_path()
        # 
        if not leaderboard_path.exists():
            leaderboard_path.mkdir()
            return
        # 
        for path in leaderboard_path.iterdir():
            if path.suffix.lower() == ".json":
                individual = Individual.load(path)
                self._leaderboard_data.append(self._LeaderEntry(individual))
        heapq.heapify(self._leaderboard_data)

    def _update_leaderboard(self, individual, score):
        path = self.get_leaderboard_path()
        heapq.heappush(self._leaderboard_data, self._LeaderEntry(individual))
        save_this_individual = True
        while len(self._leaderboard_data) > self.leaderboard:
            (_score, neg_asc) = heapq.heappop(self._leaderboard_data)
            path.joinpath(str(-neg_asc) + ".json").unlink(missing_ok=True)
            if neg_asc == -individual.ascension:
                save_this_individual = False
        if save_this_individual:
            individual.save(path)

    def get_leaderboard(self):
        """
        The leaderboard is a list of pairs of (path, score).
        It is sorted descending so leaderboard[0] is the best individual.
        """
        self._leaderboard_data.sort()
        return [(self.path.joinpath(str(-neg_asc) + ".json"), score)
                for (score, neg_asc) in reversed(self._leaderboard_data)]

    def get_best(self):
        """
        Returns the best individual who has ever died.

        Only available if the leaderboard is enabled.
        Returns None if the leaderboard is empty.
        """
        if not self.leaderboard:
            raise ValueError("leaderboard is disabled")
        if not self._leaderboard_data:
            return None
        (_score, neg_asc) = max(self._leaderboard_data)
        path = self.get_leaderboard_path().joinpath(str(-neg_asc) + ".json")
        best = Individual.load(path)
        return best

    def _load_hall_of_fame(self):
        1/0 # TODO

    def _update_hall_of_fame(self, individual, score):
        1/0

    def get_num_deaths(self) -> int:
        return self.ascension_counter

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
        self._controller    = _clean_ctrl_command(controller)
        self._path          = Path(path).expanduser().resolve()
        self._select        = select
        self._score         = score
        self._population    = []
        self._scores        = []
        self._buffer        = []

    def path(self):
        return self._path

    def get_population(self):
        """
        Returns a list of file paths.
        """
        self._scan()
        return self._population

    def birth(self, parents):
        self._scan()
        if not self._buffer:
            indices = self._select.select(128, self._scores)
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

class Evolution(API):
    """
    This class implements several standard evolutionary algorithms.

    This class does not manipulate the genomes. That work is delegated to the
    user provided functions: "crossover", "mutate", and "seed".
    This class treat genomes as opaque blobs of JSON data.

    The population is the set of all individuals who are eligible to mate. All
    individuals in the population are dead and should have been assigned a
    score, which represents their reproductive fitness. This class supports the
    following population management strategies:

    Generation:
        Manage the population in batches. Each new generation replaces the
        previous generation, entirely and all at once. This is the default.

    Continuous:
        Continuously add new individuals to the population by replacing the
        oldest member.

    Maximizing:
        Continuously add new individuals to the population by replacing the
        lowest scoring individual in the population.
    """
    def __init__(self, controller, seed, mutate=None, crossover=None, allow_mating=True,
                 path=None,
                 population_type="generation",
                 population_size=1000,
                 elites=0,
                 select=None,
                 score="score"):
        """
        Argument controller is the command line invocation for the controller program.

        Argument seed is the initial genetic material to begin evolution from.
                 It can be either a JSON-encodable object,
                 or a function which returns a JSON encodable object.

        Argument mutate is an optional function for transforming the genome:
                    f(genome) -> genome
                 It is called on every new individual before they are born.
                 This argument defaults to the identity function.

        Argument crossover is an optional function for merging multiple parent
                 genomes into a child genome. If missing then this class can
                 only perform asexual reproduction.

        Argument allow_mating controls whether this class respects "Mate"
                 requests from the environment. If "allow_mating" is True and
                 the "birth" method is given parents, then those parents used
                 instead of sampling from the population. If False then
                 the "birth" method ignores any given parents and samples the
                 population.

        Argument path is an optional directory for saving the working state of
                 this evolution service.

        Argument population_type is one of "generation", "continuous", or "maximizing".

        Argument population_size is the maximum number of individuals allowed in
                 the mating pool at once.

        Argument elites is the number of high scoring individuals to be cloned
                 (without modification) into each new generation.

        Argument select is a mate selection algorithm.
                 See the `mate_selection` package for more information.

        Argument score is an optional custom scoring function.
        """
        # Clean up and save the arguments.
        self.controller     = _clean_ctrl_command(controller)
        self.seed           = seed
        self.mutate         = mutate
        self.crossover      = crossover
        assert callable(self.mutate) or self.mutate is None
        assert callable(self.crossover) or self.crossover is None
        self.allow_mating   = bool(allow_mating)
        if path is not None:
            path = Path(path)
            self.path       = path.joinpath("population")
        else:
            self._tempdir   = tempfile.TemporaryDirectory()
            self.path       = Path(self._tempdir.name)
        # 
        self.ascension_counter = 0
        self.path.mkdir(parents=True, exist_ok=True)
        # 
        if   population_type == "generation":   PopClass = _Population
        elif population_type == "continuous":   PopClass = _Continuous
        elif population_type == "maximizing":   PopClass = _Maximizing
        else: raise ValueError("unrecognized population type")
        self._population = PopClass(
            self.path, select, score, population_size, elites)

    def get_path(self):
        """
        Returns the "path" argument.
        If the path was missing then this will return a temporary directory.
        """
        return self.path

    def get_controller(self):
        """
        Returns the "controller" argument.
        """
        return self.controller

    def birth(self, parents):
        """"""
        if self.allow_mating and len(parents):
            pass # Environment has already selected the parents.
        else:
            # Evolutionary algorithm will select the parents.
            if self.get_generation() > 1:
                parents = self._population.sample()
                parents = [Individual.load(path) for path in parents]
            else:
                if callable(self.seed):
                    seed = self.seed()
                else:
                    seed = copy.deepcopy(self.seed)
                if not isinstance(seed, Individual):
                    seed = Individual(genome=seed)
                parents = [seed]

        # Sexual reproduction
        if self.crossover is not None:
            genome = self.crossover(parents)
        else:
            # Asexual Reproduction, randomly select one of the parents to clone.
            path = random.choice(parents)
            genome = Individual.load(path).get_genome()

        # 
        if self.mutate is not None:
            genome = self.mutate(genome)

        return Individual(
                genome=genome,
                controller=self.controller,
                parents=len(parents))

    def _assign_ascension(self, individual):
        if individual.ascension is None:
            individual.ascension = self.ascension_counter
            self.ascension_counter += 1
        else:
            self.ascension_counter = max(self.ascension_counter, individual.ascension + 1)

    def death(self, individual):
        """"""
        self._assign_ascension(individual)
        self._population.death(individual)

    def get_generation(self):
        """
        Returns the number of complete generations that have fully died.
        """
        return int(self.ascension_counter / self._population.size)

class _Population:
    """
    Manages a population of individuals using regular generations.
    """
    def __init__(self, path, select, score, size, elites):
        self.path   = Path(path)
        self.select = select
        self.score  = score
        self.size   = int(size)
        self.elites = int(elites)
        assert self.path.exists()
        assert self.size >= 0
        assert self.elites >= 0
        self._scan()

    EntryType = collections.namedtuple("Entry", ("score", "ascension", "path"))

    def Entry(self, individual) -> EntryType:
        """ Class Constructor """
        return self.EntryType(
            individual.get_ascension(),
            individual.get_custom_score(self.score),
            individual.get_path())

    def _scan(self):
        if getattr(self, "_scan_time", -1) == getmtime(self.path):
            return
        self._buffer = []
        self.data = [self.Entry(individual) for individual in self.scan_dir(self.path)]
        self.sort()
        self.rollover()
        self._scan_time = getmtime(self.path)

    def sort(self):
        self.data.sort(key=lambda entry: entry.ascension)
        self.data = collections.deque(self.data)

    @staticmethod
    def scan_dir(directory):
        for path in Path(directory).iterdir():
            if path.suffix.lower() == ".json":
                individual = Individual.load(path)
                yield individual

    def death(self, individual):
        self._scan()
        individual.save(self.path)
        self._scan_time = getmtime(self.path)
        self.data.append(self.Entry(individual))
        self.rollover()

    def rollover(self):
        while len(self.data) >= 2 * self.size:
            for _ in range(self.size):
                individual = self.data.popleft()
                individual.path.unlink()
            self._buffer.clear()

    def sample(self) -> ['Path', 'Path']:
        self._scan()
        # 
        if self._buffer:
            return self._buffer.pop()
        # 
        if not self.data:
            return None
        # 
        scores = [x.score for x in self.data][:self.size]
        paths  = [x.path  for x in self.data][:self.size]
        pairs  = self.select.pairs(128, scores)
        self._buffer = [(paths[a], paths[b]) for a,b in pairs]
        return self._buffer.pop()

class _Continuous(_Population):
    def rollover(self):
        while len(self.data) > self.size:
            individual = self.data.popleft()
            individual.path.unlink()
            self._buffer.clear()

class _Maximizing(_Population):
    def sort(self):
        heapq.heapify(self.data)

    def death(self, individual):
        min_score = self.data[0].score
        low_score = individual.score <= min_score
        pop_full  = len(self.data) >= self.size
        if low_score and pop_full:
            return

        individual.save(self.path)
        heapq.heappush(self.Entry(individual))
        self.rollover()

    def rollover(self):
        while len(self.data) > self.size:
            individual = heapq.heappop()
            individual.path.unlink()
            self._buffer.clear()
