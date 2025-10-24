# The Evolution Interface #

This chapter describes the interfaces for genetic and evolutionary algorithms.
An "**individual**" refers to a distinct life-form with its own genome.
Evolutionary algorithms operate on individuals, while genetic algorithms
operate on genomes.


## The Individual File Format ##

Individuals are stored in a standard file format. An individual consist of a
genome and a bundle of metadata. The genome is stored as a binary blob; the
metadata is stored as a JSON object. Individuals are required to have a name,
controller, and genome, everything else is optional. Unexpected JSON attributes
are allowed and accessible through the python and rust APIs. The following
table defines the standard metadata attributes:

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

Individual files always named after the individual, and with the file extension `.indiv`


## The Evolution API ##

The evolution API defines a service which gives out individuals upon request.
Each instance of the evolution API is responsible for exactly one population, so
environments with multiple populations will need multiple instances. Each
population evolves independently. One instance can serve multiple environments.

The evolution API has two methods: spawn and death, which mark the beginning and
end of an individual's life cycle.


### Spawn ###

The spawn method generates new individuals. It may create genomes by any method
it sees fit.

_method signature:_ `evolution.spawn(self) -> individual`


### Death ###

The death method notifies the evolutionary algorithm that an individual that it
spawned has died. Environments should call this method when an AI agent dies.

_method signature:_ `evolution.death(self, individual)`

The death method can assume that all of the individuals it is given were
produced by the same instance's birth method. The birth method can **not**
assume that all of the individuals it produces will eventually be given to the
death method.

