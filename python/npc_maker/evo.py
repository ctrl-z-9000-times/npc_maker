"""
Evolution API, for making and using evolutionary algorithms.
"""

from pathlib import Path
import json
import shlex
import uuid

__all__ = (
    "Individual",
    "Evolution",
    # "main_loop",
    # "Remote",
)

def _clean_ctrl_command(command):
    if isinstance(command, str):
        command = shlex.split(command)
    else:
        command = list(command)
    program = Path(command[0]).expanduser().resolve()
    command[0] = program
    return command

class Individual:
    """
    Container for a distinct life-form, its controller, and all of its associated data.
    """
    def __init__(self, population, controller, genome):
        """
        Argument population is the name of population which contains this individual.

        Argument controller is the command line invocation for the controller program

        Argument genome is an object which is JSON encode-able.
        """
        self.name       = str(uuid.uuid4())
        self.population = str(population)
        self.controller = _clean_ctrl_command(controller)
        self.genome     = genome
        self.score      = None
        self.info       = {}
        self.parents    = 0
        self.children   = 0

    def get_population(self):
        """ Get the name of this individual's population. """
        return self.population

    def get_name(self):
        """ Get this individual's name, which is a UUID. """
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
        """
        return self.parents

    def get_children(self):
        """
        How many children does this individual have?
        """
        return self.children

class Evolution:
    """
    Abstract class for implementing evolutionary algorithms
    and other parameter optimization techniques.

    Users should inherit from this class and implement its methods.
    Then pass an instance of the class to "npc_maker.evo.main_loop()".

    Users may also implement custom serialization/deserialization using the
    __getstate__ and __setstate__ methods, for the server auto-save feature.
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

        Note: individual's lose their name when they die.
        """
        raise TypeError("abstract method called")

def main_loop(evolution, ip, port):
    """
    Start an evolution HTTP server listening. This never returns!

    Argument evolution implements the Evolution interface: "npc_maker.evo.Evolution".

    Example Usage:
    >>> if __name__ == "__main__":
    >>>     npc_maker.evo.main_loop( MyEvolution() )
    """
    if issubclass(evolution, Evolution):
        evolution = evolution()
    assert isinstance(evolution, Evolution)

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

class Remote(Evolution):
    """
    Connect to an evolution server over HTTP.
    """
    def __init__(self, url):
        """
        Argument url is ... TODO
        """
        import requests
        self.url = url

    def get_url(self):
        """ Get the "url" argument. """
        return self.url

    def birth(self, population, parents):
        parents = [p.get_genome() if isinstance(obj, Individual) else p for p in parents]
        url = self.url + "/" + population + "/birth"
        response = requests.post(url, json=parents)
        if not response.ok:
            raise RuntimeError(response.reason)
        ctrl, genome, info = response.json()
        return (ctrl, genome, info)

    def death(self, individual):
        genome = individual.get_genome()
        info   = individual.get_info()
        score  = individual.get_score()
        url = self.url + "/" + population + "/death"
        response = requests.post(url, json=[genome, info, score])
        if not response.ok:
            raise RuntimeError(response.reason)
