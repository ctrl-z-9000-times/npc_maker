# The Evolution Interface #

The evolution API defines a service which gives out genomes upon request.
Genomes are the parameters for AI agents. This API supports evolutionary
algorithms as well as most other parameter optimization techniques.

Each instance of the evolution API is responsible for exactly one population, so
environments with multiple populations will need multiple instances. Each
population evolves independently. One instance can serve multiple environments.

The evolution API has two methods: birth and death, which mark the beginning and
end of an individual's life cycle.


## Birth ##

The birth method generates new genomes. Environments may suggest specific
parents to mate together, however the birth method may choose to ignore the
given parents. The birth method may generate genomes by any method it sees fit.
The returned individual must have a genome and a controller, other attributes
are optional.

_method signature:_ `instance.birth(self, parents: list-of-individuals) -> individual`


## Death ##

The death method removes an individual from an environment and gives it
back to the evolution API instance that created it. Environments should call
this method when an AI agent dies.

_method signature:_ `instance.death(self, individual)`

The death method can assume that given individuals originated from the same
instance's birth method. The birth method can **not** assume that all of the
individuals it produces will eventually be given to the death method.


## Individuals ##

The "**individual**" contains a genome and an associated bundle of metadata.
The genome is stored as a binary blob; the metadata is stored as a JSON object.
Unexpected metadata is allowed and preserved in python in the attribute `Individual.extra`.
The following table defines the standard metadata attributes:

| Attribute  | JSON Type | Description |
| :--------  | :-------: | :---------- |
| `"name"`        | String    | UUID of this individual |
| `"ascension"`   | Number    | Number of individuals who died before this one |
| `"environment"` | String    | Name of the environment that this individual lives in |
| `"population"`  | String    | Name of this population that this individual belongs to |
| `"controller"`  | List of Strings | Command line invocation of the controller program |
| `"score"`       | String    | Reproductive fitness of this individual, as assessed by the environment |
| `"telemetry"`   | Map of Strings to Strings | The environmental info dictionary |
| `"epigenome"`   | Map of Strings to Strings | The epigenetic info dictionary |
| `"parents"`     | Number    | Number of parents |
| `"children"`    | Number    | Number of children |
| `"generation"`  | Number    | Number of generations that came before this individual |
| `"birth_date"`  | String    | UTC timestamp |
| `"death_date"`  | String    | UTC timestamp |

The file format for individuals is:
1) The metadata, as a utf-8 JSON formatted string
2) A single NULL character, \x00
3) The genome, as a binary array until end of file

