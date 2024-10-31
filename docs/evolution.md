# The Evolution Interface

The evolution interface defines a service which gives out genomes upon request.
Genomes are the parameters for AI agents. The evolution service can make genomes
using any method it sees fit, as long as the controller program understands them.
In addition to evolutionary algorithms, this interface can also support most other
parameter optimization techniques.

The evolution interface uses the REST architecture. Implementations can be used
either directly from python, or by starting an HTTP server.
One evolution service instance can serve multiple environments.
Each instance is responsible for exactly one population, so environments with
multiple populations will need to contact multiple evolution services.


## API Methods

The evolution interface has three methods:


### `controller() -> command`

Returns the command line invocation for the controller program.
This is a list of strings, where the first string is the filesystem path of the
controller program, and the remaining strings are its command line arguments.

All members of a population will use the same controller command.


### `birth(parents) -> [genome, info]`

Argument `parents` is a list of genomes. Optional.  
The evolution service may choose to ignore the parents and generate
genomes by any method it sees fit.  
Evolutionary algorithms typically do the following, depending on how many parents they are given:
| Number of Parents | Response |
| :---------------: | :------- |
|   0  | Create a new seed denovo |
|   1  | Reproduce asexually      |
|  >1  | Reproduce sexually       |

Returns a pair of (genome, info) where:
* genome is a JSON-encodable object,
* info is a dictionary of arbitrary string key-value pairs.  
  The environment can update this info and it will be returned to 
  evolution service when the individual dies.


### `death(individual)`

Environments use this method to notify the evolution service when an agent dies.

Argument `individual` is a JSON object containing the following information:

| Attribute  | JSON Type | Description |
| :--------  | :-------: | :---------- |
| population | String    | The name of a population |
| genome     | Object    | The genetic parameters for the AI agent |
| score      | String    | An optional message from the environment |
| info       | Mapping of Strings to Strings | The info dictionary, with all of the environment's updates |
| parents    | Number    | Putative number of parents * |
| children   | Number    | Putative number of children * |

\* The number of parents and children reported assumes that the `birth` method
is actually using the "parents" argument to produce children.

The `death` method can assume that all genomes originated from the `birth` method.

The `birth` method can **not** assume that all of the genomes it produces will
eventually be given to the `death` method. For example if an environment
crashes then all of its genomes are lost and forgotten about.


## HTTP Interface

[MEMO: This section is unimplemented]

The NPC Maker provides an HTTP server for hosting evolution services.
Each service is mounted in a directory named after its population.

| Path | Method | Arguments | Returns |
| :--- | :----- | :-------- | :------ |
| `/POPULATION/controller` | `GET`  |  | The command line invocation |
| `/POPULATION/birth` | `GET`  | The parents list | Pair of [genome, info] |
| `/POPULATION/death` | `POST` | The individual object |  | 

