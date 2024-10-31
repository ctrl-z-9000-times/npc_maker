"""
Evolution API, for making and using evolution services.
"""

from pathlib import Path
import heapq
import json
import math
import shlex
import tempfile
import uuid

__all__ = (
    "Individual",
    "Service",
    # "main_loop",
    # "Remote",
    "Recorder",
    "Replayer",
    "Evolution",
    # "Neat",
)

def _clean_ctrl_command(command):
    if isinstance(command, str):
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

def _get_score(individual, score_function):
    """
        Argument score is an optional custom scoring function. It must be one of:
                 * A callable function: f(individual) -> float,
                 * The word "score",
                 * The word "ascension",
                 * A key in the individual's info dictionary.
    """
    if callable(score_function):
        return score_function(individual)
    elif not score_function or score_function == "score":
        return individual.score
    elif score_function == "ascension":
        if individual.ascension is None:
            return math.nan
        else:
            return individual.ascension
    elif score_function in individual.info:
        return individual.info[score_function]
    else:
        raise ValueError("unrecognized score function " + repr(score_function))

class Individual:
    """
    Container for a distinct life-form and all of its associated data.
    """
    def __init__(self, environment, population, controller, genome):
        """
        Argument environment is the name of environment which contains this individual.

        Argument population is the name of population which contains this individual.

        Argument controller is the command line invocation for the controller program

        Argument genome is an object which is JSON encode-able.
        """
        self.name           = str(uuid.uuid4())
        self.environment    = str(environment)
        self.population     = str(population)
        self.controller     = _clean_ctrl_command(controller)
        self.genome         = genome
        self.score          = None
        self.info           = {}
        self.parents        = 0
        self.children       = 0
        self.birth_date     = None
        self.death_date     = None
        self.ascension      = None

    def get_environment(self):
        """ Get the name of this individual's environment. """
        return self.environment

    def get_population(self):
        """ Get the name of this individual's population. """
        return self.population

    def get_name(self):
        """
        Get this individual's name, which is a UUID.

        Note: individual's lose their name when they die.
        """
        return self.name

    def get_controller(self):
        """ Get the command line invocation for the controller program. """
        return self.controller

    def get_genome(self):
        """
        Get this individual's genetic data.
        Returns a bundle of decoded JSON data (a python object).
        """
        return self.genome

    def get_score(self):
        """ Get the most recently assigned score, or None if it has not been assigned yet. """
        return self.score

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

    def save(self, path):
        """
        Serialize this individual to JSON and write it to a file.

        Argument path is the directory to save to.

        The filename will be either the individual's name or its ascension number,
        And the file extension will be ".json"

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
        data = {
            "name":        self.name,
            "environment": self.environment,
            "population":  self.population,
            # "controller":  self.controller,
            "genome":      self.genome,
            "info":        self.info,
            "parents":     self.parents,
            "children":    self.children,
        }
        # Optional fields.
        if self.score is not None:      data["score"]      = self.score
        if self.ascension is not None:  data["ascension"]  = self.ascension
        if self.birth_date is not None: data["birth_date"] = self.birth_date
        if self.death_date is not None: data["death_date"] = self.death_date
        # 
        with open(path, 'wt') as file:
            json.dump(data, file)
        return path

    def load(path, controller):
        """
        Load a previously saved individual.
        """
        with open(path, 'rt') as file:
            data = json.load(file)
        # 
        individual = Individual(
                data.pop("environment"),
                data.pop("population"),
                controller,
                data.pop("genome"))
        # Load optional fields.
        individual.name       = data.pop("name",       individual.name)
        individual.score      = data.pop("score",      individual.score)
        individual.info       = data.pop("info",       individual.info)
        individual.parents    = data.pop("parents",    individual.parents)
        individual.children   = data.pop("children",   individual.children)
        individual.ascension  = data.pop("ascension",  individual.ascension)
        individual.birth_date = data.pop("birth_date", individual.birth_date)
        individual.death_date = data.pop("death_date", individual.death_date)
        # Load any extra fields in case the user wants them.
        individual.__dict__.update(data)
        return individual

class Service:
    """
    Abstract class for implementing evolutionary algorithms
    and other parameter optimization techniques.

    Users should inherit from this class and implement its methods.
    Then pass an instance of the class to "npc_maker.evo.main_loop()".

    Users may also implement custom serialization/deserialization using the
    __getstate__ and __setstate__ methods, for the server's auto-save feature.
    """
    def controller(self) -> [str]:
        """
        Abstract Method

        Returns the command line invocation for the controller program.
        """
        raise TypeError("abstract method called")

    def birth(self, parents) -> 'Genome':
        """
        Abstract Method

        Generate and return a new genome, by any means deemed appropriate.

        Argument parents is a list of genomes.

        Returns the genome, which is a python object that is JSON encode-able.
        """
        raise TypeError("abstract method called")

    def death(self, individual):
        """
        Abstract Method

        Notification of an individual's death.
        """
        raise TypeError("abstract method called")

def main_loop(services, ip, port):
    """
    Start an evolution HTTP web-server listening. This never returns!

    Argument services is a dictionary of .... implement the Service interface: "npc_maker.evo.Service".

    Example Usage:
    >>> if __name__ == "__main__":
    >>>     npc_maker.evo.main_loop( MyService() )
    """
    if issubclass(evolution, Service):
        evolution = evolution()
    assert isinstance(evolution, Service)

    from flask import Flask

    app = Flask(__name__)

    @app.route("/")
    def hello_world():
        return "<p>Hello, World!</p>"

    app.run()

    # Listen on the given IP & port
    1/0 # TODO

    # TODO: Choose a server implementation:
    #   waitress: https://docs.pylonsproject.org/projects/waitress/en/stable/index.html
    #   mod_wsgi: https://www.modwsgi.org/en/develop/

class Remote(Service):
    """
    Connect to an evolution server over HTTP.
    """
    def __init__(self, url, population):
        """
        Argument url is ... TODO
        """
        import requests
        self.url = url
        self.population = population

    def get_url(self):
        """ Get the "url" argument. """
        return self.url

    def controller(self):
        url = self.url + "/" + population + "/controller"
        response = requests.get(url)
        if not response.ok:
            raise RuntimeError(response.reason)
        return response.json

    def birth(self, population, parents):
        parents  = [p.get_genome() if isinstance(obj, Individual) else p for p in parents]
        url      = self.url + "/" + self.population + "/birth"
        response = requests.post(url, json=parents)
        if not response.ok:
            raise RuntimeError(response.reason)
        ctrl, genome, info = response.json()
        return (ctrl, genome, info)

    def death(self, individual):
        genome = individual.get_genome()
        info   = individual.get_info()
        score  = individual.get_score()
        url    = self.url + "/" + self.population + "/death"
        response = requests.post(url, json=[genome, info, score])
        if not response.ok:
            raise RuntimeError(response.reason)

class Recorder(Service):
    """
    This wrapper records recording high scoring individuals and statistics from
    an evolution services
    """
    def __init__(self, service, path=None, leaderboard=None, hall_of_fame=None,
                 score="score", filters={}):
        """
        Argument service is the underlying evolution service to record from.

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

        Argument score is an optional custom scoring function. It must be one of:
                 * A callable function: f(individual) -> float,
                 * The word "score" (the default),
                 * The word "ascension",
                 * A key in the individual's info dictionary.

        Argument filters is a dictionary of custom filter functions for selecting
                 which individuals to save. Each key-value pair defines a new filter.
                 * The key is the name of the directory to save to.
                 * The value is a callable function: f(individual) -> bool,
                   where returning True will save the individual, False will reject it.
        """
        """
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
        self.hall_of_fame   = int(hall_of_fame) if hall_of_fame is not None else 0
        self.score          = score
        self.filters        = dict(filters)
        # self.statistics     = dict(statistics)
        # self.histograms     = dict(histograms)

        assert isinstance(service, Service)
        assert self._path.is_dir()
        assert self.leaderboard >= 0
        assert self.hall_of_fame >= 0

        if self.leaderboard: self._load_leaderboard()
        if self.hall_of_fame: self._load_hall_of_fame()

    def get_path(self):
        return self._path

    def get_leaderboard_path(self):
        return self._path.joinpath("leaderboard")

    def get_hall_of_fame_path(self):
        return self._path.joinpath("hall_of_fame")

    def controller(self):
        return self.service.controller(parents)

    def birth(self, parents):
        return self.service.birth(parents)

    def death(self, individual):
        self.service.death(individual)

        score = _get_score(individual, self.score)

        if self.leaderboard:  self._update_leaderboard( individual, score)
        if self.hall_of_fame: self._update_hall_of_fame(individual, score)

        for filter_name, filter_function in self.filters.items():
            if filter_function(individual):
                individual.save(self._path.joinpath(filter_name))

        # for statistic_name, statistic_data in self.statistics.items():
        #     1/0
        # for histogram_name, histogram_data in self.histograms.items():
        #     1/0

    def _load_leaderboard(self):
        self._leaderboard_data = []
        for path in self._path.joinpath("leaderboard").iterdir():
            if path.suffix.lower() == ".json":
                individual = Individual.load(path)
                score = _get_score(individual, self.score)
                entry = (score, -individual.get_ascension())
                self._leaderboard_data.append(entry)
        heapq.heapify(self._leaderboard_data)

    def _update_leaderboard(self, individual, score):
        path = self._path.joinpath("leaderboard")
        entry = (score, -individual.get_ascension())
        heapq.heappush(self._leaderboard_data, entry)
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
        TODO
        """
        if not self.leaderboard:
            raise ValueError("leaderboard is disabled")
        if not self._leaderboard_data:
            return None
        (score, neg_asc) = max(self._leaderboard_data)
        path = self.path.joinpath(str(-neg_asc) + ".json")
        best = Individual.load(path)
        return best

    def _load_hall_of_fame(self):
        1/0 # TODO

    def _update_hall_of_fame(self, individual, score):
        1/0

    def get_num_deaths(self) -> int:
        return self.ascension_counter

    def get_generation(self) -> int:
        """
        Returns the current generation number, staring at zero.
        Returns None if the generation size was not specified.
        """
        if self.hall_of_fame:
            return int(self.ascension_counter / self.hall_of_fame)
        else:
            return None

class Replayer(Service):
    """
    Replay saved individuals as an evolution service
    """
    def __init__(self, controller, path, select="Random", score="score"):
        """
        Argument controller is the command line invocation for the controller program.

        Argument path is the directory containing the saved individuals.
                 Each individual must be a ".json" file.

        Argument select is a mate selection algorithm.

        Argument score is an optional custom scoring function. It must be one of:
                 * A callable function: f(individual) -> float,
                 * The word "score" (the default),
                 * The word "ascension",
                 * A key in the individual's info dictionary.
        """
        self._controller    = _clean_ctrl_command(controller)
        self._path          = Path(path).expanduser().resolve()
        self._select        = select
        self._score         = score
        self._population    = []
        self._scores        = []
        self._scan_time     = -1
        self._buffer        = []

    def controller(self):
        return self._controller

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
        if os.path.getmtime(self._path) == self._scan_time:
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
        self._scores = []
        for path in self._population:
            individual = Individual.load(path)
            score = _get_score(individual, self._score)
            self._scores.append(score)

class Evolution(Service):
    """
    This class implements several common and simple evolutionary algorithms.

    This class does not manipulate the genomes. That work is delegated to the
    user provided functions: "crossover", "mutate", and "seed".
    This class treat genomes as opaque blobs of JSON data.

    The population is the set of all individuals who are eligible to mate.
    All individuals in the population are dead and were assigned a valid score,
    which represents their reproductive fitness. This class supports the
    following population management strategies:

    Generation:
        Manage the population in batches. Each new generation replaces the
        previous generation, entirely and all at once.

    Continuous:
        Continuously add new individuals to the population by replacing the
        oldest member.

    Best:
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

        Argument allow_mating (Boolean, default: True) controls whether this
                 class respects "Mate" requests from the environment.
                 If "allow_mating" is True and the "birth" method is given
                 parents, then those parents used instead of sampling from the
                 population. If False then any parents given to the "birth"
                 method are always ignored and the population is always sampled.

        Argument path is an optional directory for saving the working state of
                 this evolution service.

        Argument population_type is one of: "generation", "continuous", or "best".

        Argument population_size is the maximum number of individuals allowed in
                 the mating pool at once.

        Argument elites is the number of high scoring individuals to be cloned
                 (without modification) into each new generation.

        Argument select is a mate selection algorithm.

        Argument score is an optional custom scoring function. It must be one of:
                 * A callable function: f(individual) -> float,
                 * The word "score" (the default),
                 * The word "ascension",
                 * A key in the individual's info dictionary.
        """
        # Clean up and save the arguments.
        self._controller        = _clean_ctrl_command(controller)
        self._seed              = seed
        self._mutate            = mutate
        self._crossover         = crossover
        self._allow_mating      = bool(allow_mating)
        self._path              = Path(path).joinpath("population") if path is not None else None
        self._population_type   = str(population_type).strip().lower()
        self._population_size   = int(population_size)
        self._elites            = int(elites)
        assert callable(self._mutate) or self._mutate is None
        assert callable(self._crossover) or self._crossover is None
        assert self._population_type in ("generation", "continuous", "best")
        assert self._population_size >= 0
        assert self._elites          >= 0
        # 
        self._ascension_counter = 0
        self._path.mkdir(parents=True, exist_ok=True)
        self._load_population()

    def _load_population(self):
        self._population = []
        if self._population_type == "generation":
            1/0
        elif self._population_type == "continuous":
            1/0
        elif self._population_type == "best":
            1/0

    def get_path(self):
        return self._path

    def controller(self):
        return self._controller

    def birth(self, parents):
        if self._allow_mating and len(parents):
            1/0
            return [genome, {}]

        # Get the scores for the population.
        1/0

        # Select one or two parents from the population.
        if crossover is None:
            1/0
        else:
            1/0

        # 
        if mutate is not None:
            genome = mutate(genome)

        return [genome, {}]

    def _assign_ascension(self, individual):
        if not hasattr(individual, "ascension"):
            individual.ascension = self.ascension_counter
            self.ascension_counter += 1
        else:
            self.ascension_counter = max(self.ascension_counter, individual.ascension + 1)

    def death(self, individual):
        self._assign_ascension(individual)
        # 
        if self._population_type == "generation":
            1/0

            individual.save(self._path)

        elif self._population_type == "continuous":
            1/0
        elif self._population_type == "best":
            1/0
        else:
            raise RuntimeError()

    def get_generation(self):
        1/0

class Neat(Service):
    """
    """
    def __init__(self):
        """
        """
        1/0

    def controller(self):
        return self._controller

    def birth(self, parents):
        1/0

    def death(self, individual):
        1/0
