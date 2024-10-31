# The Evolution Interface #

The evolution interface defines a service which gives out genomes upon request.
Genomes are the parameters for AI agents. The evolution service can make genomes
using any method it sees fit, as long as the controller program understands them.
In addition to evolutionary algorithms, this interface supports most other
parameter optimization techniques.

The evolution interface uses the REST architecture. Implementations can be used
either directly from python, or by starting an HTTP web-server. Either way,
each instance is referred to as an evolution "**service**". Each evolution
service is responsible for exactly one population, so environments with
multiple populations will need to contact multiple evolution services. One
evolution service can serve multiple environments.


## Evolution Service API ##

Each evolution service instance must have these three methods:

### Method: `controller() -> command` ###

Returns the command line invocation for the controller program. The command line
invocation is a list of strings, where the first string is the filesystem path
of the controller program, and the remaining strings are its command line arguments.

All members of a population will use the same controller command line invocation.

### Method: `birth(parents) -> [genome, info]` ###

Argument `parents` is a list of genomes.

Depending on how many parents are provided, evolutionary algorithms typically do
the following:

| Number of Parents | Suggested Response |
| :---------------: | :----------------- |
|   0  | Create a new seed denovo |
|   1  | Reproduce asexually      |
|  >1  | Reproduce sexually       |

However, the evolution service may choose to ignore the parents altogether and
generate genomes by any method it sees fit.

The birth method returns a pair of (genome, info) where:
* genome is a JSON-encodable object,
* info is a dictionary of arbitrary string key-value pairs.
  The environment can update this info and it will be returned to 
  evolution service when the individual dies.

### Method: `death(individual)` ###

Environments use this method to notify the evolution service when an agent dies.

Argument `individual` is a JSON object containing the following information:

| Attribute  | JSON Type | Description |
| :--------  | :-------: | :---------- |
| `"environment"` | String    | The name of the environment which individual lived in |
| `"population"`  | String    | The name of this population. This is a key into the environment specification's "populations" table |
| `"genome"`      | Anything  | The genetic parameters from the AI agent |
| `"score"`       | String    | Represents the reproductive fitness of the individual, as assessed by the environment. Optional |
| `"info"`        | Mapping of Strings to Strings | The info dictionary, with all of the environment's updates |
| `"parents"`     | Number    | Putative number of parents * |
| `"children"`    | Number    | Putative number of children * |
| `"birth_date"`  | String    | UTC timestamp taken by the management process |
| `"death_date"`  | String    | UTC timestamp taken by the management process |
| `"ascension"`   | Number    | Number of individuals who died before this one. This attribute is **missing** from the JSON object. Evolution services may assign the ascension number |

\* The reported number of parents and children assumes that the `birth` method
is actually using its "parents" argument to produce children.

The `death` method can assume that all genomes originated from the `birth` method.

The `birth` method can **not** assume that all of the genomes it produces will
eventually be given to the `death` method. For example if an environment program
crashes then all of its genomes are going to be lost and forgotten about.


## HTTP Interface ##

[MEMO: This section is unimplemented]

The NPC Maker provides an HTTP server for hosting evolution services.
Each service is mounted in a directory named after its population.

| Path | Method | Arguments | Returns |
| :--- | :----- | :-------- | :------ |
| `/POPULATION/controller` | `GET`  |  | The command line invocation |
| `/POPULATION/birth` | `GET`  | The parents list | Pair of [genome, info] |
| `/POPULATION/death` | `POST` | The individual object |  | 

