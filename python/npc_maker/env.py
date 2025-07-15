"""
Environment Interface, for making and using environments.

All global functions in this module are for implementing environment programs.
"""

from pathlib import Path
import npc_maker.ctrl
import npc_maker.evo
import collections
import datetime
import json
import os
import subprocess
import sys
import time

__all__ = (
    "death",
    "Environment",
    "eprint",
    "get_args",
    "telemetry",
    "mate",
    "new",
    "poll",
    "score",
    "SoloAPI",
    "Specification",
)

def _timestamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(" ", 'milliseconds')

def Specification(env_spec_path):
    """
    Load an environment specification from file.
    """
    # Clean up the filesystem path argument.
    env_spec_path = Path(env_spec_path).expanduser().resolve()
    # Read the file into memory.
    with open(env_spec_path, 'rt') as env_spec_file:
        env_spec_data = env_spec_file.read()
    # Parse the file into a JSON object.
    try:
        env_spec = json.loads(env_spec_data)
    except json.decoder.JSONDecodeError as err:
        raise ValueError(f"JSON syntax error in \"{env_spec_path}\" {err}")
    # 
    _env_spec_check_fields(env_spec, ("name", "path", "populations",), ("spec",))
    # Automatically save the env_spec path into the env_spec.
    env_spec["spec"] = env_spec_path
    # Clean up the environment command line invocation.
    env_path = Path(env_spec["path"])
    # Environment program paths are relative to the env_spec file.
    if not env_path.is_absolute():
        env_path = env_spec_path.parent.joinpath(env_path)
    # env_path = env_path.resolve()
    env_spec["path"] = env_path
    # Allow certain undocumented abbreviations.
    _alias_fields(env_spec, [
        ("desc", "description"),
        ("descr", "description"),
        ("population", "populations"),])
    # Insert default values for missing keys.
    if "settings"    not in env_spec: env_spec["settings"]    = []
    if "populations" not in env_spec: env_spec["populations"] = []
    if "description" not in env_spec: env_spec["description"] = ""
    # Check first level data types.
    assert isinstance(env_spec["name"], str)
    assert isinstance(env_spec["populations"], list)
    assert isinstance(env_spec["settings"], list)
    assert isinstance(env_spec["description"], str)
    # Check population objects.
    # assert len(env_spec["populations"]) > 0
    for pop in env_spec["populations"]:
        _env_spec_check_fields(pop, ("name",))
        _alias_fields(pop, [
            ("desc", "description"),
            ("descr", "description"),])
        # Insert default values for missing keys.
        if "interfaces"  not in pop: pop["interfaces"]  = []
        if "description" not in pop: pop["description"] = ""
        # Check the population's data types.
        assert isinstance(pop["name"], str)
        assert isinstance(pop["interfaces"], list)
        assert isinstance(pop["description"], str)
        # Check the interface objects.
        for interface in pop["interfaces"]:
            _env_spec_check_fields(interface, ("gin", "name",))
            _alias_fields(interface, [
                ("desc", "description"),
                ("descr", "description"),])
            if "description" not in interface: interface["description"] = ""
            assert isinstance(interface["name"], str)
            assert isinstance(interface["gin"], int)
            assert isinstance(interface["description"], str)
        # Check interface names are unique.
        interface_names = [interface["name"] for interface in pop["interfaces"]]
        if len(interface_names) != len(set(interface_names)):
            raise ValueError("duplicate interface names in population specification")
    # Check population names are unique.
    population_names = [pop["name"] for pop in env_spec["populations"]]
    if len(population_names) != len(set(population_names)):
        raise ValueError("duplicate population name in environment specification")
    # Check settings objects.
    for item in env_spec["settings"]:
        _clean_settings(item)
    # Check settings names are unique.
    settings_names = [item["name"] for item in env_spec["settings"]]
    if len(settings_names) != len(set(settings_names)):
        raise ValueError("duplicate settings name in environment specification")
    # 
    return env_spec

def _env_spec_check_fields(json_object, require_fields=(), reserved_fields=()):
    # Check that it's a JSON object.
    if not isinstance(json_object, dict):
        raise ValueError(f"expected a JSON object in environment specification")
    # Check for required and forbidden keys.
    for field in require_fields:
        if field not in json_object:
            raise ValueError(f'missing field "{field}" in environment specification')
    for field in reserved_fields:
        if field in json_object:
            raise ValueError(f'reserved field "{field}" in environment specification')

def _alias_fields(json_object, aliases):
    for (abrv, attr) in aliases:
        if abrv in json_object:
            if attr in json_object:
                raise ValueError(
                    f"duplicate fields: \"{abrv}\" and \"{attr}\" in environment specification")
            json_object[attr] = json_object.pop(abrv)

def _clean_settings(item):
    """ Settings items are strictly / rigidly structured. """
    _env_spec_check_fields(item, ("name", "type", "default",))
    num_fields = 3

    _alias_fields(item, [
        ("desc", "description"),
        ("descr", "description"),])
    item["description"] = item.get("description", "")
    num_fields += 1

    # Normalize the type aliases.
    if   item["type"] == "float": item["type"] = "Real"
    elif item["type"] == "int":   item["type"] = "Integer"
    elif item["type"] == "bool":  item["type"] = "Boolean"
    elif item["type"] == "enum":  item["type"] = "Enumeration"
    assert item["type"] in ("Real", "Integer", "Boolean", "Enumeration")

    # Clean each type variant.
    if item["type"] == "Boolean":
        item["default"] = bool(item["default"])

    elif item["type"] in ("Real", "Integer"):
        _env_spec_check_fields(item, ("minimum", "maximum",))
        num_fields += 2
        if item["type"] == "Real":
            item["default"] = float(item["default"])
            item["minimum"] = float(item["minimum"])
            item["maximum"] = float(item["maximum"])
        elif item["type"] == "Integer":
            item["default"] = int(item["default"])
            item["minimum"] = int(item["minimum"])
            item["maximum"] = int(item["maximum"])
        assert item["minimum"] <= item["default"]
        assert item["maximum"] >= item["default"]

    elif item["type"] == "Enumeration":
        _env_spec_check_fields(item, ("values",))
        num_fields += 1
        item["default"] = str(item["default"])
        item["values"]  = [str(variant) for variant in item["values"]]
        assert len(item["values"]) == len(set(item["values"]))
        assert item["default"] in item["values"]

    if len(item) > num_fields:
        name = item["name"]
        raise ValueError(
            f"unexpected attributes on setting \"{name}\" in environment specification")

def _cast_env_settings(env_spec, settings):
    """ Cast the command line argument settings to the data type specified in the environment specification. """
    settings_list = env_spec.get("settings")
    if settings_list is None:
        return
    settings_dict = {spec["name"]: spec for spec in settings_list}
    for name, value in settings.items():
        if spec := settings_dict.get(name):
            data_type = spec.get("type")
            if data_type == "Real" or data_type == "float":
                settings[name] = float(value)
            elif data_type == "Integer" or data_type == "int":
                settings[name] = int(value)
            elif data_type == "Boolean" or data_type == "bool":
                if isinstance(value, str):
                    value = value.lower()
                    if   value == "false": value = False
                    elif value == "true":  value = True
                settings[name] = bool(value)
            elif data_type == "Enumeration" or data_type == "enum":
                settings[name] = str(value)

def _help_message(env_spec):
    # Usage.
    pass

    # Title and description.
    message = env_spec["name"] + " Environment\n\n"
    desc = env_spec.get("description", "")
    if desc:
        message += desc + "\n\n"

    # Summary of populations.
    pass

    # Summary of command line arguments.
    settings = env_spec.get("settings", [])
    if settings:
        name_field      = max(len(item["name"]) for item in settings)
        default_field   = max(len(str(item["default"])) for item in settings)
        name_field      = max(name_field,    len("Argument"))
        default_field   = max(default_field, len("Default"))
        message += f"Type | Argument | Default | Range (inclusive) | Description \n"
        message +=  "-----+----------+---------+-------------------+-------------\n"
        for item in settings:
            if   item["type"] == "Real":        line = "real | "
            elif item["type"] == "Integer":     line = "int  | "
            elif item["type"] == "Boolean":     line = "bool | "
            elif item["type"] == "Enumeration": line = "enum | "
            line += item["name"].ljust(name_field) + " | "
            line += str(item["default"]).ljust(default_field) + " | "
            if item["type"] == "Real" or item["type"] == "Integer":
                line += str(item["minimum"]) + " - " + str(item["maximum"])
            elif item["type"] == "Enumeration":
                line += ", ".join(item["values"])
            line += " | "
            line += item["description"]
            message += line + "\n"
    return message

class Environment:
    """
    This class encapsulates an instance of an environment and provides methods
    for using environments.

    Each environment instance execute in its own subprocess
    and communicates with the caller over its standard I/O channels.
    """
    def __init__(self, populations, env_spec, mode='graphical', settings={}, stderr=sys.stderr):
        """
        Start running an environment program.

        Argument populations is a dict of evolution API instances, indexed by population name.
                 Every population must have a corresponding instance of npc_maker.evo.Evolution.

        Argument env_spec is the filesystem path of the environment specification.

        Argument mode is either the word "graphical" or the word "headless" to
                 indicate whether or not the environment should show graphical
                 output to the user.

        Argument settings is a dict of command line arguments for the environment process.
                 These must match what is listed in the environment specification.

        Argument stderr is the file descriptor to use for the subprocess's stderr channel.
                 By default, the controller will inherit this process's stderr channel.
        """
        self.outstanding = {}
        # Load the environment specification from file.
        self.env_spec = Specification(env_spec)
        # Special case for when there is exactly one population.
        populations_spec = self.env_spec["populations"]
        if len(populations_spec) == 1:
            if isinstance(populations, npc_maker.evo.Evolution):
                populations = {populations_spec[0]["name"]: populations}
        # Clean the populations argument.
        self.populations = dict(populations)
        all_population_names = set(pop["name"] for pop in populations_spec)
        assert set(self.populations.keys()) == all_population_names
        assert all(isinstance(instance, npc_maker.evo.Evolution) for instance in self.populations.values())
        # Clean the display mode argument.
        self.mode = str(mode).strip().lower()
        assert self.mode in ('graphical', 'headless')
        # Clean the settings argument
        settings = {str(key) : str(value) for key,value in settings.items()}
        # Fill in default settings values and check for extra arguments.
        settings_spec = self.env_spec["settings"]
        self.settings = {item["name"] : item["default"] for item in settings_spec}
        for key, value in settings.items():
            if key not in self.settings:
                raise ValueError(f"unrecognized environment setting \"{key}\"")
            self.settings[key] = value
        # Assemble the environment's optional settings.
        settings_list = []
        for key, value in self.settings.items():
            settings_list.append(str(key))
            settings_list.append(str(value))
        # 
        self._process = subprocess.Popen(
            [self.env_spec["path"], self.env_spec["spec"], self.mode] + settings_list,
            stdin  = subprocess.PIPE,
            stdout = subprocess.PIPE,
            stderr = stderr)
        os.set_blocking(self._process.stdout.fileno(), False)

    def is_alive(self):
        """
        Check if the environment subprocess is still running or if it has exited.
        """
        return self._process.poll() is None

    def __del__(self):
        if hasattr(self, "_process"): # Guard against crashes in __init__.
            self.quit()
        self._kill_outstanding()

    def get_populations(self):
        """
        Get the "populations" argument.
        """
        return self.populations

    def get_env_spec(self):
        """
        Get the environment specification.
        This returns the loaded JSON object, *not* its filesystem path.
        """
        return self.env_spec

    def get_mode(self):
        """
        Get the output display "mode" argument.
        """
        return self.mode

    def get_settings(self):
        """
        Get the "settings" argument.
        """
        return dict(self.settings)

    def get_outstanding(self):
        """
        Get all individuals who are currently alive in this environment.
        Returns a dictionary indexed by individuals names.
        """
        return self.outstanding

    def _kill_outstanding(self):
        """
        Return all outstanding individuals back to the evolutionary algorithm.
        This effectively abandons them in the environment.
        """
        for individual in self.outstanding.values():
            population_name = individual.get_population()
            self.populations[population_name].death(individual)
        self.outstanding.clear()

    def quit(self):
        """
        Tell the environment program to exit.
        """
        self._process.stdin.close()

    def _birth(self, individual, parents):
        """
        Send an individual to the environment.
        Individuals must not be birthed more than once.
        Does not flush.
        """
        # Unpack the individual's data.
        assert isinstance(individual, npc_maker.evo.Individual)
        name    = individual.get_name()
        pop     = individual.get_population()
        parents = [p.get_name() for p in parents]
        genome  = individual.get_parameters()
        ctrl    = individual.get_controller()
        if ctrl is None:
            raise ValueError("indiviual is missing controller")
        ctrl[0] = str(ctrl[0]) # Convert Path to String
        assert isinstance(genome, bytes)
        # Process the request.
        self.outstanding[name] = individual
        self._process.stdin.write('{{"name":"{}","population":"{}","parents":{},"controller":{},"genome":{}}}\n'
            .format(name, pop, json.dumps(parents), json.dumps(ctrl), len(genome))
            .encode("utf-8"))
        self._process.stdin.write(genome)
        individual.birth_date = _timestamp()

    def poll(self):
        """
        Check for messages from the environment program.

        This function is non-blocking and should be called periodically.
        """
        def make_child(population_name, parents):
            population = self.populations[population_name]
            child = population.birth(parents)
            if not isinstance(child, npc_maker.evo.Individual):
                child = npc_maker.evo.Individual(**child)
            child.environment   = self.env_spec["name"]
            child.population    = population_name
            return child

        # Limit the number of messages received to avoid blocking the main thread.
        for _ in range(100):
            # Check for messages.
            message = self._process.stdout.readline().strip()
            if not message:
                # Flush all queued responses on the way out the door.
                self._process.stdin.flush()
                return

            # Decode the message.
            message = json.loads(message)

            if "New" in message:
                population = message["New"]
                if population is None:
                    all_populations = self.env_spec["populations"]
                    if len(all_populations) == 1:
                        population = all_populations[0]["name"]
                    else:
                        raise ValueError("missing field \"populations\"")
                parents = []
                child = make_child(population, parents)
                self._birth(child, parents)

            elif "Mate" in message:
                parents = message["Mate"]
                parents = [self.outstanding[p] for p in parents]
                population = parents[0].get_population()
                assert all(p.get_population() == population for p in parents)
                child = make_child(population, parents)
                self._birth(child, parents)

            elif "Score" in message:
                score       = message["Score"]
                name        = message["name"]
                individual  = self.outstanding[name]
                individual.score = score

            elif "Telemetry" in message:
                info        = message["Telemetry"]
                name        = message["name"]
                individual  = self.outstanding[name]
                individual.telemetry.update(info)

            elif "Death" in message:
                name                    = message["Death"]
                individual              = self.outstanding.pop(name)
                individual.deathdate    = _timestamp()
                individual.name         = None
                population_name         = individual.get_population()
                self.populations[population_name].death(individual)

            else:
                raise ValueError(f'unrecognized message "{message}"')

    @classmethod
    def run(cls, individuals, env_spec, mode='graphical', settings={}, stderr=sys.stderr):
        """
        Evaluate the given individuals in the given environment.

        Argument individuals is a dictionary indexed by population name.
                 Each entry is an iterable of individuals.

        The remaining arguments are for the Environment class constructor.

        Returns an identical data structure except that the iterables are
        replaced with lists of the evaluated individuals.
        """
        outstanding = 0 # Birth count - death count.
        exhausted = False # Has at least one iterator ended?
        class Dispatcher(npc_maker.evo.Evolution):
            def __init__(self, individuals):
                self.iter = iter(individuals)
                self.ascended = []
            def birth(self, parents):
                nonlocal outstanding
                child = next(self.iter)
                outstanding += 1
                return child
            def death(self, individual):
                nonlocal outstanding
                self.ascended.append(individual)
                outstanding -= 1
        dispatchers = {population_name: Dispatcher(indiv_list)
                        for population_name, indiv_list in individuals.items()}
        env = cls(dispatchers, env_spec, mode, settings, stderr=stderr)
        while env.is_alive():
            if outstanding <= 0 and exhausted:
                break
            try:
                env.poll()
            except StopIteration:
                exhausted = True
                continue
            time.sleep(0.1)
        return {population_name: population.ascended
                for population_name, population in dispatchers.items()}

def eprint(*args, **kwargs):
    """
    Print to stderr

    The NPC Maker uses the environment program's stdin & stdout to communicate
    with the main program via a standardized JSON-based protocol. Unformatted
    diagnostic and error messages should be written to stderr using this function.
    """
    print(*args, **kwargs, file=sys.stderr, flush=True)

def get_args():
    """
    Read the command line arguments for an NPC Maker environment program.

    Returns a tuple of (environment-specification, graphics-mode, settings-dict)
    Environment programs *must* call this function for initialization purposes.
    """
    os.set_blocking(sys.stdin.fileno(), False)
    # 
    def error(message):
        eprint(message)
        sys.exit(1)
    # Read the command line arguments.
    if len(sys.argv) < 2:
        error("missing argument: environment specification")
    program = sys.argv[0]
    # Read the environment specification file.
    try:
        env_spec = Specification(sys.argv[1])
    except Exception as err:
        error(err)
    # Print help message and exit.
    if '-h' in sys.argv or '--help' in sys.argv:
        error(_help_message(env_spec))
    # Read the graphics mode.
    if len(sys.argv) >= 3:
        mode  = sys.argv[2].strip().lower()
    else:
        mode  = 'graphical' # Default setting.
    # Check the graphics mode.
    if mode not in ['graphical', 'headless']:
        error(f"argument error: expected either \"graphical\" or \"headless\", got \"{mode}\"")
    # Read the user's settings.
    settings = sys.argv[3:]
    if len(settings) % 2 == 1:
        error("argument error: odd number of settings, expected key-value pairs")
    settings = zip(settings[::2], settings[1::2])
    # Overwrite the default values with the user's settings.
    defaults = {item['name']: item['default'] for item in env_spec.get('settings', [])}
    for item, value in settings:
        if item not in defaults:
            error(f"argument error: unexpected parameter \"{item}\"")
        defaults[item] = value
    _cast_env_settings(env_spec, defaults)
    return (env_spec, mode, defaults)

def poll():
    """
    Check for messages from the evolution program.

    Callers *must* call the `get_args()` function before using this,
    for initialization purposes.

    This function is non-blocking and will return None if there are no new messages. 
    If the standard input channel is closed then this returns the string "Quit".
    Otherwise this decodes JSON messages into python objects and returns them.
    """
    try:
        message = sys.stdin.readline()
    # If any communication channels with the main program are dead then exit immediately.
    except ValueError:
        if sys.stdin.closed:
            return "Quit"
        else:
            raise
    except EOFError:
        return "Quit"
    # Ignore empty lines.
    message = message.strip()
    if not message:
        sys.stdout.flush()
        sys.stderr.flush()
        if sys.stdin.closed:
            return "Quit"
        return None
    # Decode the JSON string into a python object.
    try:
        message = json.loads(message)
    except json.decoder.JSONDecodeError as err:
        eprint(f"JSON syntax error in \"{message}\" {err}")
        return None
    # 
    num_bytes = int(message["genome"])
    genome = sys.stdin.read(num_bytes)
    message["genome"] = genome
    # 
    return message

def _try_print(*args, **kwargs):
    # If the stdout channel is simply closed, then quietly exit.
    # For other more abnormal conditions raise the error to the user.
    # 
    # Closing stdin will cause all future calls to poll() to return the "Quit" message.
    try:
        print(*args, **kwargs, file=sys.stdout, flush=True)
    except BrokenPipeError:
        sys.stdin.close()
    except ValueError:
        if sys.stdout.closed:
            sys.stdin.close()
        else:
            raise

def new(population=None):
    """
    Request a new individual from this population's evolution API.

    Argument population is optional if the environment has exactly one population.
    """
    if population is not None:
        population = str(population)
    _try_print(json.dumps({"New": population}))

def mate(*parents):
    """
    Request to mate specific individuals together to produce a child individual.
    """
    parents = [str(p) for p in parents]
    assert len(parents) > 0
    _try_print(json.dumps({"Mate": parents}))

def score(name, score):
    """
    Report an individual's score or reproductive fitness to the evolution API.

    This should be called *before* calling "death()" on the individual.
    """
    name = str(name)
    score = str(score)
    _try_print(json.dumps({"Score": score, "name": name}))

def telemetry(name, info):
    """
    Report extra information about an individual.

    Argument info is a mapping of string key-value pairs.
    """
    name = str(name)
    info = {str(key) : str(value) for key, value in info.items()}
    _try_print(json.dumps({"Telemetry": info, "name": name}))

def death(name):
    """
    Notify the evolution API that the given individual has died.

    The individual's score or reproductive fitness should be reported
    using the "score()" function *before* calling this method.
    """
    name = str(name)
    _try_print(json.dumps({"Death": name}))

class SoloAPI:
    """
    Abstract class for implementing environments which contain exactly one
    individual at a time. The environment must have exactly one population.
    """
    def __init__(self, env_spec, mode, **settings):
        """
        Abstract Method, Optional

        Environments are initialized with these arguments:

        Argument env_spec is the environment specification, as a python object.
                 It is already loaded from file, parsed, and error checked.

        Argument mode is either the word "graphical" or the word "headless" to
                 indicate whether or not the environment should show graphical
                 output to the user.

        Additional keyword argument are the environment's settings, as described
        by the environment specification. All of the settings will be provided,
        using either their default values or they can be overridden via command
        line arguments as key-value pairs.
        """

    def advance(self, name, controller):
        """
        Abstract Method

        Advance the state of the environment by one discrete time step.

        Argument name is the UUID string of the individual who is currently
                 occupying the environment.

        Argument controller is an instance of "npc_maker.ctrl.Controller"

        Returns the individual's score or None. If a score is returned then the
        individual is dead and a new individual will be created for the next
        call to the advance() method. If None is returned then the environment
        is still evaluating the current individual and the same individual
        (name and controller) will be passed to the next call to advance().
        """
        raise TypeError("abstract method called")

    def idle(self):
        """
        Abstract Method, Optional

        This is called periodically while the environment is Paused or Stopped.
        Environments should refrain from computationally intensive workloads while idling.
        """
        pass

    def quit(self):
        """
        Abstract Method, Optional

        Called just before the environment exits.
        """
        pass

    @classmethod
    def main(cls, buffer=1):
        """
        Run the environment program.

        Argument buffer is the number of individuals to request at once.

        This function handles communications between the environment
        (this program) and the evolution program, which execute in separate
        computer processes and communicate over the environment's standard I/O
        channels.

        This never returns!

        Example Usage:
        >>> if __name__ == "__main__":
        >>>     MyEnvironment.main()
        """
        assert buffer >= 1
        env_spec, mode, settings = get_args()
        assert len(env_spec["populations"]) == 1
        self = cls(env_spec, mode, **settings)
        population = env_spec["populations"][0]["name"]
        controller = None
        cache = {} # command -> controller
        queue = collections.deque()

        def is_running():
            return queue or (controller is not None)

        for _ in range(buffer):
            new(population)

        # Main Program Loop.
        while True:

            # Message Read Loop.
            while request := poll():
                if request == "Quit":
                    break

                queue.append(request)

            if not is_running():
                self.idle()
                idle_fps = 30
                time.sleep(1 / idle_fps) # Don't excessively busy loop.

            else:
                # Birth New Controller.
                if controller is None and queue:
                    request    = queue.popleft()
                    name       = request["name"]
                    command    = request["controller"]
                    # Reuse controller instances if able.
                    if controller is None:
                        controller = cache.get(tuple(command))
                    # Start a new controller process.
                    if controller is None:
                        controller = npc_maker.ctrl.Controller(env_spec, population, command)
                        cache[tuple(command)] = controller
                    assert controller.is_alive()
                    controller.genome(request["genome"])

                # Advance Controller One Step.
                if controller is not None:
                    final_score = self.advance(name, controller)
                    if final_score is not None:
                        score(name, final_score)
                        death(name)
                        controller = None
                        new(population)
        self.quit()
