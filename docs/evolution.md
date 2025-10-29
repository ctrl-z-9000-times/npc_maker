# The Evolution Interface #

This chapter describes the interfaces for genetic and evolutionary algorithms.
An "**individual**" refers to a distinct life-form with its own genome.
Evolutionary algorithms operate on individuals, while genetic algorithms
operate on genomes.


## The Individual File Format ##

Individuals are stored in a standard file format. An individual consist of a
genome and a bundle of metadata. The genome is stored as a binary blob; the
metadata is stored as a JSON object. Individuals are required to have a name,
controller, and genome; everything else is optional. Unexpected JSON attributes
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

The purpose of the evolutionary algorithm is to pick which individuals to
reproduce. The evolution API has two methods: spawn and death, which mark the
beginning and end of an individual's life cycle.

One instance of the evolution API can serve multiple environments.
Each instance of the API is responsible for exactly one population, so
environments with multiple populations will need multiple instances.


### Spawn ###

_method signature:_ `evolution.spawn(self) -> [individual]`

The spawn method returns a list of parent individuals to be mated together to
produce a child. Depending on how many parents are returned take the following
actions:

| Parents | Action |
| --- | --- |
| 0 | Use the initial genetic material |
| 1 | Asexually reproduce the parent |
| 2 | Sexually reproduce the parents |
| 3+ | Unspecified |


### Death ###

_method signature:_ `evolution.death(self, individual)`

The death method notifies the evolutionary algorithm that an individual has died.


## The Genetics API ##

Whereas the evolutionary algorithm decides which individuals to reproduce, the
genetics module reproduces the genomes. The word "**genome**" refers a complete
set of parameters for creating an AI agent. Each individual has exactly one
immutable genome. When an individual is born into an environment its genome is
converted into a "**phenome**" before transmission to the control system.

The genetics API is not well defined because it is too tightly coupled with the
user's evolutionary algorithm. The experimental setup will determine which
functions are required, and additional functions may be necessary. The genetics
API defines the following functions:


### asex ###

_function signature:_ `asex(parent_genome) -> (child_genome, child_phenome)`

This function asexually reproduces a genome. It should create a copy of the
given genome and apply mutations.


### sex ###

_function signature:_ `sex(parent_1_genome, parent_2_genome) -> (child_genome, child_phenome)`

This function sexually reproduces two genomes. It should apply crossover to
combine the given genomes and then apply mutations.


### phenome ###

_function signature:_ `phenome(parent_genome) -> child_phenome`

Package the genome into a binary object in preparation for sending it to the
control system.


### distance ###

_function signature:_ `distance(parent_1_genome, parent_2_genome) -> f64`

Used by the NEAT algorithm for artificial speciation.

