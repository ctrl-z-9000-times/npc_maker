"""
Controller API, for making and using control systems.
"""

from pathlib import Path
import errno
import json
import shlex
import subprocess
import sys

__all__ = (
    "Controller",
    "API",
    "eprint",
    "get_args",
    "main_loop",
)

def eprint(*args, **kwargs):
    """
    Print to stderr

    The NPC Maker uses the controller's stdin & stdout for communication using a
    standardized message protocol. Unformatted diagnostic and error messages
    should be written to stderr with this function.
    """
    print(*args, **kwargs, file=sys.stderr, flush=True)

def _clean_ctrl_command(command):
    if isinstance(command, str):
        command = shlex.split(command)
    else:
        command = list(command)
    program = Path(command[0]).expanduser().resolve()
    command[0] = program
    return command

class Controller:
    """
    An instance of a controller.

    This class provides methods for using controllers.

    Each controller instance is executed in a subprocesses.
    """
    def __init__(self, environment, population, command, stderr=sys.stderr):
        """
        Argument environment is the file path of the current environment specification file.

        Argument population is a name and a key into the environment spec's "populations" table.

        Argument command is the command line invocation for the controller program.
                 It may either be a string, or a list or strings in which case the first
                 value is the program and the remaining strings are its command line arguments.

        Argument stderr is the file descriptor to use for the subprocess's stderr channel.
                 By default, the controller will inherit this process's stderr channel.
        """
        if isinstance(environment, dict):
            environment = environment["spec"]
        self.environment    = Path(environment)
        self.population     = str(population)
        self.command        = _clean_ctrl_command(command)
        self._ctrl          = subprocess.Popen(self.command,
            stdin           = subprocess.PIPE,
            stdout          = subprocess.PIPE,
            stderr          = stderr)
        # 
        self._ctrl.stdin.write("E{}\n".format(self.environment).encode("utf-8"))
        self._ctrl.stdin.write("P{}\n".format(self.population).encode("utf-8"))

    def get_environment(self):
        """ Get the "environment" argument. """
        return self.environment

    def get_population(self):
        """ Get the "population" argument. """
        return self.population

    def get_command(self):
        """ Get the "command" argument. """
        return " ".join(str(arg) for arg in self.command)

    def __repr__(self):
        return "<npc_maker.env_api.Instance: {}>".format(repr(self.get_command()))

    def new(self, genome):
        """
        Initialize the control system with a new genome.
        This discards the currently loaded model.

        The genome should already be encoded in JSON.
        For example:
        >>> import json
        >>> genome = json.dumps(genome)
        """
        self._ctrl.stdin.write("N{}\n".format(genome).encode("utf-8"))

    def reset(self):
        """
        Reset the control system to its initial state.
        """
        self._ctrl.stdin.write(b"R\n")

    def advance(self, dt):
        """
        Advance the control system's internal state.

        Argument dt is in units of seconds.
        """
        dt = float(dt)
        self._ctrl.stdin.write("X{}\n".format(dt).encode("utf-8"))

    def set_input(self, gin, value):
        """
        Write a single value to a GIN in the controller.
        """
        value = str(value)
        gin   = int(gin)
        assert '\n' not in value
        assert gin >= 0
        self._ctrl.stdin.write("I{}:{}\n".format(gin, value).encode("utf-8"))

    def set_binary(self, gin, binary):
        """
        Write an array of bytes to a GIN in the controller.
        """
        # binary = bytes(binary)
        gin = int(gin)
        assert isinstance(binary, bytes)
        assert gin >= 0
        self._ctrl.stdin.write("B{}:{}\n".format(gin, len(binary)).encode("utf-8"))
        self._ctrl.stdin.write(binary)

    # TODO: Make a new version of get_outputs that is split into request/receive phases
    #       so that users can get outputs from many controllers all at once.

    # TODO: Add a timeout for reading from the controller's stdin.

    def get_outputs(self, gin_list):
        """
        Retrieve a list of outputs, as identified by their GIN.

        This method blocks on IO.
        """
        return_list = True
        if hasattr(gin_list, "__iter__"):
            gin_list    = list(gin_list)
        else:
            gin_list    = [gin_list]
            return_list = False
        # Request the outputs.
        for gin in gin_list:
            gin = int(gin)
            assert gin >= 0
            self._ctrl.stdin.write("O{}\n".format(gin).encode("utf-8"))
        self._ctrl.stdin.flush()
        # Receive the outputs.
        outputs = {}
        while len(outputs) < len(gin_list):
            message = self._ctrl.stdout.readline().lstrip()
            if not message: continue
            gin, value   = message.split(b":", maxsplit=1)
            gin          = int(gin)
            outputs[gin] = value.decode("utf-8")
        # assert set(outputs) == set(gin_list)
        if return_list:
            return outputs
        else:
            return outputs.popitem()[1]

    def save(self, path):
        """
        Save the current state of the controller to file.
        """
        path = Path(path)
        self._ctrl.stdin.write("S{}\n".format(path).encode("utf-8"))
        self._ctrl.stdin.flush()

    def load(self, path):
        """
        Load the state of the controller from file.
        """
        path = Path(path)
        self._ctrl.stdin.write("L{}\n".format(path).encode("utf-8"))

    def message(self, message_type, message_body):
        """
        Send a custom message to the controller using a new message type.
        """
        message_type = str(message_type).strip().upper()
        assert len(message_type) == 1
        assert message_type not in "EPNRXIBOSLQ"
        self._ctrl.stdin.write("{}{}\n".format(message_type, message_body).encode("utf-8"))

    def quit(self):
        """
        Stop running the controller process.
        """
        self._ctrl.stdin.write(b"Q\n")
        self._ctrl.stdin.flush()
        self._ctrl.stdin.close()

    def __del__(self):
        if hasattr(self, "_ctrl"):
            try:
                self.quit()
            except IOError as error:
                if error.errno == errno.EPIPE:
                    pass

class API:
    """
    Abstract class for implementing controllers.

    Controllers should inherit from this class and implement all of its methods.
    Then call "npc_maker.ctrl.main_loop()" with an instance of your class to
    run an instance of your controller program.
    """
    def new(self, genome: str):
        """ Abstract Method """
        raise TypeError("abstract method called")

    def reset(self):
        """ Abstract Method """
        raise TypeError("abstract method called")

    def advance(self, dt: float):
        """ Abstract Method """
        raise TypeError("abstract method called")

    def set_input(self, gin: int, value: str):
        """ Abstract Method """
        raise TypeError("abstract method called")

    def set_binary(self, gin: int, binary: bytes):
        """ Abstract Method """
        raise TypeError("unsupported operation")

    def get_output(self, gin: int) -> str:
        """ Abstract Method """
        raise TypeError("abstract method called")

    def save(self, path: Path):
        """ Abstract Method """
        raise TypeError("unsupported operation")

    def load(self, path: Path):
        """ Abstract Method """
        raise TypeError("unsupported operation")

    def message(self, message_type: str, message_body: str):
        """ Abstract Method """
        raise TypeError(f"unsupported operation (message type \"{message_type}\")")

    def quit(self):
        """ Abstract Method, Optional """
        pass

_stdin       = open(sys.stdin.fileno(),  mode='rb')
_buffer      = b""
_environment = None
_population  = None

def _readline():
    global _stdin, _buffer
    if b"\n" not in _buffer:
        while True:
            chunk = _stdin.read1()
            _buffer += chunk
            if b"\n" in chunk:
                break
            if _stdin.closed:
                raise EOFError("stdin closed")
    line, _buffer = _buffer.split(b"\n", maxsplit=1)
    line = line.decode("utf-8")
    return line

def _parse_message():
    # Ignore leading white space and empty lines.
    while True:
        message = _readline()
        message = message.lstrip()
        if message:
            break
        # 
    msg_type = message[0].upper()
    msg_body = message[1:]
    return (msg_type, msg_body)

def get_args():
    """
    Returns pair of (environment, population)

    The environment is the file path of the current environment specification file.
    The population is a name and a key into the environment spec's "populations" table.

    This information is always available to the controller process
    and does not change during the controller process's lifetime.
    """
    global _environment, _population
    while _environment is None or _population is None:
        msg_type, msg_body = _parse_message()
        if msg_type == "E":
            _environment = Path(msg_body)
        elif msg_type == "P":
            _population = msg_body
        else:
            raise RuntimeError("protocol error: missing environment or population")
    return _environment, _population

def main_loop(controller):
    """
    Start the main loop of the controller program.

    Argument controller implements the controller interface: "npc_maker.ctrl.API"

    This function handles communications between the controller (this program) and
    the environment. It reads and parses messages from stdin, interfaces with
    your implementation of the controller API, and writes messages to stdout.

    This function never returns!

    Example Usage:
    >>> if __name__ == "__main__":
    >>>     npc_maker.ctrl.main_loop( MyController() )
    """
    global _stdin, _environment, _population
    if type(controller) is type and issubclass(controller, API):
        controller = controller()
    assert isinstance(controller, API)
    while True:
        msg_type, msg_body = _parse_message()

        if msg_type == "I":
            gin, value = msg_body.split(":", maxsplit=1)
            gin = int(gin)
            controller.set_input(gin, value)

        elif msg_type == "O":
            gin   = int(msg_body)
            value = str(controller.get_output(gin))
            assert '\n' not in value
            reply = f"{gin}:{value}"
            try:
                print(reply, flush=True)
            except ValueError:
                if sys.stdout.closed:
                    raise EOFError("stdout closed")

        elif msg_type == "B":
            gin, num_bytes  = msg_body.split(":")
            gin             = int(gin)
            num_bytes       = int(num_bytes)
            binary          = _stdin.read(num_bytes)
            if len(binary) != num_bytes:
                raise EOFError("stdin closed")
            controller.set_binary(gin, binary)

        elif msg_type == "X":
            dt = float(msg_body)
            controller.advance(dt)

        elif msg_type == "R":
            controller.reset()

        elif msg_type == "N":
            genome = json.loads(msg_body)
            controller.new(genome)

        elif msg_type == "E":
            _environment = Path(msg_body)

        elif msg_type == "P":
            _population = msg_body

        elif msg_type == "S":
            save_path = Path(msg_body)
            controller.load(save_path)

        elif msg_type == "L":
            load_path = Path(msg_body)
            controller.load(load_path)

        elif msg_type == "Q":
            controller.quit()
            break

        else:
            controller.message(msg_type, msg_body)
