"""
Environment API, for making and using environments.

All global functions in this module are for implementing environment programs.
"""

from .evo import Individual, Evolution
from pathlib import Path
import json
import os
import shlex
import subprocess
import sys
import time

__all__ = (
    "Environment",
    # "Remote",
    "eprint",
    "get_args",
    "poll",
    "ack",
    "new",
    "mate",
    "score",
    "info",
    "death",
)

# TODO: Make load_env_spec into a public function. It's useful and it does a lot
# of cleanup, regularization, and error checking.

def _load_env_spec(env_spec_path):
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
    env_spec["path"] = _clean_env_command(env_spec_path, env_spec["path"])
    # Insert default values for missing keys.
    if "settings"    not in env_spec: env_spec["settings"]    = []
    if "description" not in env_spec: env_spec["description"] = ""
    # Check first level data types.
    assert isinstance(env_spec["name"], str)
    assert isinstance(env_spec["spec"], Path)
    assert isinstance(env_spec["path"], list)
    assert isinstance(env_spec["path"][0], Path)
    assert all(isinstance(arg, str) for arg in env_spec["path"][1:])
    assert isinstance(env_spec["populations"], list)
    assert isinstance(env_spec["settings"], list)
    assert isinstance(env_spec["description"], str)
    # Check population objects.
    assert len(env_spec["populations"]) > 0
    for pop in env_spec["populations"]:
        _env_spec_check_fields(pop, ("name",))
        # Insert default values for missing keys.
        if "interfaces"  not in pop: pop["interfaces"]  = []
        if "description" not in pop: pop["description"] = ""
        # Check the population's data types.
        assert isinstance(pop["name"], str)
        assert isinstance(pop["interfaces"], list)
        assert isinstance(pop["description"], str)
        # Check interface objects.
        for interface in pop["interfaces"]:
            _env_spec_check_fields(interface, ("gin", "name",))
            if "description" not in interface: interface["description"] = ""
            assert isinstance(interface["name"], str)
            assert isinstance(interface["gin"], int)
            assert isinstance(interface["description"], str)
    # Check population names are unique.
    population_names = [pop["name"] for pop in env_spec["populations"]]
    assert len(population_names) == len(set(population_names)), "duplicate population name in evolution specification"
    # Check settings objects.
    for item in env_spec["settings"]:
        _clean_settings(item)
    env_spec["settings"]
    # Check settings names are unique.
    settings_names = [item["name"] for item in env_spec["settings"]]
    assert len(settings_names) == len(set(settings_names)), "duplicate setting name in evolution specification"
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

def _clean_env_command(env_spec_path, command):
    # Split the command line invocation into a list of the program and its arguments.
    if isinstance(command, str):
        command = shlex.split(command)
    else:
        command = list(str(word) for word in command)
    # Environment program paths are relative to the env_spec file.
    program = Path(command[0])
    if not program.is_absolute():
        program = env_spec_path.parent.joinpath(program)
    # Reassemble and return the environment program's command line invocation.
    command[0] = program
    return command

def _clean_settings(item):
    """ Settings items are strictly / rigidly structured. """
    _env_spec_check_fields(item, ("name", "type", "default",))
    # Normalize type aliases.
    if   item["type"] == "float": item["type"] = "Real"
    elif item["type"] == "int":   item["type"] = "Integer"
    elif item["type"] == "bool":  item["type"] = "Boolean"
    elif item["type"] == "enum":  item["type"] = "Enumeration"
    assert item["type"] in ("Real", "Integer", "Boolean", "Enumeration")
    # 
    if item["type"] in ("Real", "Integer"):
        _env_spec_check_fields(item, ("minimum", "maximum",))
        # TODO: Check / cast data types for minimum & maximum.
    if item["type"] == "Enumeration":
        _env_spec_check_fields(item, ("values",))
    # TODO: Check against extra fields.

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

class Environment:
    """
    An instance of an environment.

    This class provides methods for using environments.

    Each environment instance execute in its own subprocess
    and communicates with this process over its standard I/O channels.
    """
    def __init__(self, evolution, env_spec, mode='graphical', settings={},
                 stderr=sys.stderr, timeout=None):
        """
        Start running an environment program.

        Argument evolution is a dict of Evolution instances, indexed by population name.
                 Every population must have a corresponding evolution instance.

        Argument env_spec is the filesystem path of the environment specification.

        Argument mode is either the word "graphical" or the word "headless" to
                 indicate whether or not the environment should show graphical
                 output to the user.

        Argument settings is a dict of command line arguments for the environment process.
                 These must match what is listed in the environment specification.

        Argument stderr is the file descriptor to use for the subprocess's stderr channel.
                 By default, the controller will inherit this process's stderr channel.

        Argument timeout is number of seconds to wait for a response from the
                environment before declaring it dead and raising a TimeoutError.
                Optional, if missing or None then this will wait forever.
        """
        # Load the environment specification from file.
        self.env_spec = _load_env_spec(env_spec)
        # Clean the evolution server argument.
        populations = self.env_spec["populations"]
        if len(populations) == 1:
            population = populations[0]["name"]
            if type(evolution) is type and issubclass(evolution, Evolution):
                evolution = evolution()
            if isinstance(evolution, Evolution):
                evolution = {population: evolution()}
        self.evolution = dict(evolution)
        all_population_names = set(pop["name"] for pop in populations)
        assert set(self.evolution.keys()) == all_population_names
        assert all(isinstance(evo, Evolution) for evo in self.evolution.values())
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
            self.env_spec["path"] + [self.env_spec["spec"], self.mode] + settings_list,
            stdin  = subprocess.PIPE,
            stdout = subprocess.PIPE,
            stderr = stderr)
        os.set_blocking(self._process.stdout.fileno(), False)
        # 
        self.timeout = None if timeout is None else float(timeout)
        self.watchdog = time.time()
        # 
        self.outstanding = {}

    def __del__(self):
        if hasattr(self, "_process"):
            try:
                self.quit()
                self.flush()
                self._process.stdin.close()
                self._process.stdout.close()
            except (EOFError, BrokenPipeError):
                pass
            except IOError as error:
                if error.errno == errno.EPIPE:
                    pass

    def get_evolution(self):
        """ Get the "evolution" argument. """
        return self.evolution

    def get_env_spec(self):
        """
        Get the environment specification.
        This returns the loaded JSON object, *not* its filesystem path.
        """
        return self.env_spec

    def get_mode(self):
        """ Get the output display "mode" argument. """
        return self.mode

    def get_settings(self):
        """ Get the "settings" argument. """
        return dict(self.settings)

    def get_outstanding(self):
        """
        Get all individuals who are currently alive in this environment.
        Returns a dictionary indexed by individuals names.
        """
        return self.outstanding

    def get_timeout(self):
        """ Get the "timeout" argument. """
        return self.timeout

    def is_alive(self):
        """ Check if the environment program's computer process is still executing. """
        return self._process.poll() is None

    def start(self):
        """
        Request to start the environment.
        Does not flush!
        """
        self._process.stdin.write(b'"Start"\n')

    def stop(self):
        """
        Request to stop the environment.
        Does not flush!
        """
        self._process.stdin.write(b'"Stop"\n')

    def pause(self):
        """
        Request to pause the environment.
        Does not flush!
        """
        self._process.stdin.write(b'"Pause"\n')

    def resume(self):
        """
        Request to resume the environment.
        Does not flush!
        """
        self._process.stdin.write(b'"Resume"\n')

    def quit(self):
        """
        Request to quit the environment.
        Does not flush!
        """
        self._process.stdin.write(b'"Quit"\n')

    def save(self, path):
        """
        Request to save the environment to the given path.
        Does not flush!
        """
        path = json.dumps(str(path))
        self._process.stdin.write(f'{{"Save":"{path}"}}\n'.encode('utf-8'))

    def load(self, path):
        """
        Request to load the environment from the given path.
        Does not flush!
        """
        path = json.dumps(str(path))
        self._process.stdin.write(f'{{"Load":"{path}"}}\n'.encode('utf-8'))

    def send(self, message):
        """
        Send an arbitrary JSON message to the environment.
        Does not flush!
        """
        message = json.dumps(message)
        self._process.stdin.write(f'{{"Message":{message}}}\n'.encode('utf-8'))

    def _birth(self, individual):
        """
        Send an individual to the environment.
        Does not flush!

        Individuals must not be birthed more than once.
        """
        # Unpack the individual's data.
        assert isinstance(individual, Individual)
        pop     = individual.get_population()
        name    = individual.get_name()
        ctrl    = individual.get_controller()
        ctrl[0] = str(ctrl[0])
        genome  = individual.get_genome()
        parents = individual.get_parents()
        # Process the request.
        self.outstanding[name] = individual
        self._process.stdin.write('{{"Birth":{{"population":"{}","name":"{}","controller":{},"genome":{},"parents":{}}}}}\n'
            .format(pop, name, json.dumps(ctrl), json.dumps(genome), json.dumps(parents))
            .encode("utf-8"))

    def flush(self):
        """
        Send all waiting messages to the environment.
        Flushes the environment subprocess's standard input channel.
        """
        self._process.stdin.flush()

    def poll(self):
        """
        Check for messages from the environment program.

        This function is non-blocking and should be called periodically.
        """
        while True:
            # Check for messages.
            message = self._process.stdout.readline().strip()
            if not message:
                # Check for environment timeout.
                if self.timeout:
                    elapsed_time = time.time() - self.watchdog
                    if elapsed_time > 1.5 * self.timeout:
                        raise TimeoutError("environment timed out")
                    elif elapsed_time > 0.5 * self.timeout:
                        self._process.stdin.write(b'"Heartbeat"\n')
                # Flush all queued responses on the way out the door.
                self.flush()
                return

            # Decode the message.
            message = json.loads(message)

            if "New" in message:
                pop = message["New"]
                if pop is None:
                    all_populations = self.env_spec["populations"]
                    if len(all_populations) == 1:
                        pop = all_populations[0]["name"]
                    else:
                        raise ValueError("missing field \"population\"")
                evo     = self.evolution[pop]
                ctrl    = evo.controller()
                genome  = evo.birth([])
                child   = Individual(pop, ctrl, genome)
                self._birth(child)

            elif "Mate" in message:
                parents = message["Mate"]
                parents = [self.outstanding[p] for p in parents]
                pop     = parents[0].get_population()
                assert all(p.get_population() == pop for p in parents)
                evo     = self.evolution[pop]
                ctrl    = evo.controller()
                genome  = evo.birth([p.get_name() for p in parents])
                child   = Individual(pop, ctrl, genome)
                child.parents   = len(parents)
                for p in parents:
                    p.children += 1
                self._birth(child)

            elif "Score" in message:
                score   = message["Score"]
                name    = message["name"]
                indiv   = self.outstanding[name]
                indiv.score = score

            elif "Info" in message:
                info    = message["Info"]
                name    = message["name"]
                indiv   = self.outstanding[name]
                indiv.info.update(info)

            elif "Death" in message:
                name        = message["Death"]
                indiv       = self.outstanding.pop(name)
                indiv.name  = None
                pop         = indiv.get_population()
                self.evolution[pop].death(indiv)

            elif "Ack" in message:
                inner = message["Ack"]
                if   inner == "Start":      self.on_start()
                elif inner == "Stop":       self.on_stop()
                elif inner == "Pause":      self.on_pause()
                elif inner == "Resume":     self.on_resume()
                elif inner == "Quit":       self.on_quit()
                elif "Save" in inner:       self.on_save(inner["Save"])
                elif "Load" in inner:       self.on_load(inner["Load"])
                elif "Message" in inner:    self.on_message(inner["Message"])
                elif "Birth" in inner:      pass
                else:
                    raise ValueError(f'unrecognized message "{message}"')
            else:
                raise ValueError(f'unrecognized message "{message}"')

            # Any valid activity will kick the watchdog.
            self.watchdog = time.time()

    def on_start(self):
        """
        Callback hook for subclasses to implement.
        Triggered by "ack" responses.
        """
    def on_stop(self):
        """
        Callback hook for subclasses to implement.
        Triggered by "ack" responses.
        """
    def on_pause(self):
        """
        Callback hook for subclasses to implement.
        Triggered by "ack" responses.
        """
    def on_resume(self):
        """
        Callback hook for subclasses to implement.
        Triggered by "ack" responses.
        """
    def on_quit(self):
        """
        Callback hook for subclasses to implement.
        Triggered by "ack" responses.
        """
    def on_save(self, path):
        """
        Callback hook for subclasses to implement.
        Triggered by "ack" responses.
        """
    def on_load(self, path):
        """
        Callback hook for subclasses to implement.
        Triggered by "ack" responses.
        """
    def on_message(self, message):
        """
        Callback hook for subclasses to implement.
        Triggered by "ack" responses.
        """

class Remote(Environment):
    """
    Run an instance of an environment over an SSH connection.

    The environment will execute on the remote compute.
    """
    def __init__(self, hostname, port,
                 evolution, env_spec, mode='graphical', settings={},
                 stderr=sys.stderr, timeout=None):
        1/0 # TODO

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
    program   = sys.argv[0]
    spec_path = Path(sys.argv[1]).expanduser().resolve()
    if len(sys.argv) >= 3:
        mode  = sys.argv[2].strip().lower()
    else:
        mode  = 'graphical' # Default setting.
    settings  = sys.argv[3:]
    # Read the environment specification file.
    try:
        env_spec = _load_env_spec(sys.argv[1])
    except Exception as err:
        error(err)
    # 
    if mode not in ['graphical', 'headless']:
        error(f"argument error: expected either \"graphical\" or \"headless\", got \"{mode}\"")
    # 
    if len(settings) % 2 == 1:
        error("argument error: odd number of settings, expected key-value pairs")
    settings = dict(zip(settings[::2], settings[1::2]))
    defaults = {item['name']: item['default'] for item in env_spec.get('settings', [])}
    for item, value in settings:
        if item not in defaults:
            error(f"argument error: unexpected parameter \"{item}\"")
        defaults[item] = value
    _cast_env_settings(env_spec, defaults)
    return (env_spec, mode, defaults)

def poll():
    """
    Check for messages from the management program.

    This function is non-blocking and will return "None" if there are no new
    messages. This decodes the JSON messages and returns python objects.

    Callers *must* call the `get_args()` function before using this,
    for initialization purposes.
    """
    try:
        message = sys.stdin.readline()
    # If only communication channels with the main program are dead then exit immediately.
    except ValueError:
        if sys.stdin.closed:
            return "Quit"
        else:
            raise
    except (EOFError, BrokenPipeError):
        return "Quit"
    # Ignore empty lines.
    message = message.strip()
    if not message:
        sys.stdout.flush()
        sys.stderr.flush()
        return None
    # Decode the JSON string into a python object.
    try:
        message = json.loads(message)
    except json.decoder.JSONDecodeError as err:
        eprint(f"JSON syntax error in \"{message}\" {err}")
        return None
    # 
    return message

def _try_print(*args, **kwargs):
    # If the stdout channel is simply closed, then quietly exit.
    # For other more abnormal conditions raise the error to the user.
    # 
    # Closing stdin will cause all future calls to poll() to return the "Quit" message.
    try:
        print(*args, **kwargs, file=sys.stdout, flush=True)
    except (EOFError, BrokenPipeError):
        sys.stdin.close()
    except ValueError:
        if sys.stdout.closed:
            sys.stdin.close()
        else:
            raise

def ack(message):
    """
    Acknowledge that the given message has been received and successfully acted upon.
    The environment may send ack's unprompted to signal unexpected changes.
    """
    global _quit_flag
    if "Birth" in message:
        pass # Birth messages shouldn't be acknowledged.
    else:
        assert message in ("Heartbeat","Load","Message","Pause","Quit","Resume","Save","Start","Stop")
        response = json.dumps({"Ack": message})
        _try_print(response)

def new(population=None):
    """
    Request a new individual from the evolution server.

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
    Report an individual's score or reproductive fitness to the evolution server.

    This should be called *before* calling "death()" on the individual.
    """
    name = str(name)
    score = str(score)
    _try_print(json.dumps({"Score": str(score), "name": name}))

def info(name, info):
    """
    Report arbitrary extraneous information about an individual to the NPC Maker.

    Argument info is a mapping of string key-value pairs.
    """
    name = str(name)
    info = {str(key) : str(value) for key, value in info.items()}
    _try_print(json.dumps({"Info": info, "name": name}))

def death(name):
    """
    Notify the evolution server that the given individual has died.

    If the individual had a score or reproductive fitness then it should be
    reported using the "score()" function *before* calling this method.
    """
    name = str(name)
    _try_print(json.dumps({"Death": name}))
