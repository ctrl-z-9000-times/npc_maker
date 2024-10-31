# The Evolution Interface #

The evolution interface defines a service which gives out genomes upon request.
Genomes are the parameters for AI agents. The evolution service can make genomes
using any method it sees fit, as long as the controller program understands them.
In addition to evolutionary algorithms, this interface supports most other
parameter optimization techniques.

The evolution interface uses the REST architecture. Each instance is referred to
as an evolution "**service**". Each evolution service is responsible for
exactly one population, so environments with multiple populations will need
multiple evolution services. One evolution service can serve multiple
environments.


## Individuals ##

The "**individual**" is the bundle of metadata that is associated with each
genome. Individuals are represented as JSON-encodable objects. Unexpected
attributes are allowed and preserved as able. The following table summarizes
the attributes of individual objects.

| Attribute  | JSON Type | Description |
| :--------  | :-------: | :---------- |
| `"name"`        | String    | UUID of this individual |
| `"ascension"`   | Number    | Number of individuals who died before this one |
| `"environment"` | String    | Name of the environment that this individual lives in |
| `"population"`  | String    | Name of this population that this individual belongs to |
| `"controller"`  | List of Strings | Command line invocation of the controller program |
| `"genome"`      | Anything  | Genetic parameters for this AI agent |
| `"score"`       | String    | Reproductive fitness of this individual, as assessed by the environment |
| `"info"`        | Mapping of Strings to Strings | The info dictionary, with all of the environment's accumulated updates |
| `"parents"`     | Number    | Number of parents |
| `"children"`    | Number    | Number of children |
| `"birth_date"`  | String    | UTC timestamp taken by the management process |
| `"death_date"`  | String    | UTC timestamp taken by the management process |

The available attributes change over the life cycle of an individual.
The genome is always available.


## Evolution Service API ##

Each evolution service instance must have these two methods:

Returns the command line invocation for the controller program. The command line
invocation is a list of strings, where the first string is the filesystem path
of the controller program, and the remaining strings are its command line arguments.

All members of a population will use the same controller command line invocation.

### Method: `birth(parents) -> individual` ###

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

### Method: `death(individual)` ###

Environments use this method to notify the evolution service when an agent dies.

The `death` method can assume that all genomes originated from the `birth` method.

The `birth` method can **not** assume that all of the genomes it produces will
eventually be given to the `death` method. For example if an environment
program crashes then all of the genomes contained in it will be lost.

