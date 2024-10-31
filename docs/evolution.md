# The Evolution Interface #

The evolution API defines a service which gives out genomes upon request.
Genomes are the parameters for AI agents. This API supports evolutionary
algorithms as well as most other parameter optimization techniques.

Each instance of the evolution API is responsible for exactly one population, so
environments with multiple populations will need multiple instances. Each
population evolves independently. One instance can serve multiple environments.

The evolution API has two methods: birth and death, which mark the beginning and
end of an individual's life cycle.


## Individuals ##

The "**individual**" is a bundle of metadata that is associated with each
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
| `"info"`        | Map of Strings to Strings | The info dictionary, with all of the environment's accumulated updates |
| `"parents"`     | Number    | Number of parents |
| `"children"`    | Number    | Number of children |
| `"birth_date"`  | String    | UTC timestamp taken by the management process |
| `"death_date"`  | String    | UTC timestamp taken by the management process |

The available attributes change over the life cycle of an individual, except for
the genome. The genome is always available.


## Birth ##

The birth method generates new genomes. Environments may suggest specific
parents to mate together, however the birth method may choose to ignore the
given parents. The birth method may generate genomes by any method it sees fit.
The returned individual must have a genome and a controller, other attributes
are optional.

_method signature:_ `instance.birth(self, parents: list-of-individuals) -> individual`

Open-ended evolutionary algorithms typically do the following, depending on how
many parents are provided:

| Number of Parents | Suggested Response |
| :---------------: | :----------------- |
|   0  | Create a new seed denovo |
|   1  | Reproduce asexually      |
|  >1  | Reproduce sexually       |


## Death ##

The death method is used to remove an individual from an environment and give it
back to the evolution API instance that created it. Environments should call
this method when an AI agent dies.

_method signature:_ `instance.death(self, individual)`

The death method can assume that given individuals originated from the same instance's birth method.

The birth method can **not** assume that all of the individuals it produces will
eventually be given to the death method. For example if an environment
program crashes then all of the individuals contained in it will be lost.

