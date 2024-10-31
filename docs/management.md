# The Management Program #

This chapter describes how to set up and run the NPC Maker framework.
The management program is responsible for gathering up the many peices of the
framework, assembling them together, and setting it all in motion.

See the [examples](../examples/) directory for practical guidance on using the NPC Maker's APIs.

## Example System Configurations ##

There are many possible ways for you, the user, to set up your computer systems.  
Here are a few examples:

![Minimal Configuration](images/syscfg_minimal.svg)

![Environments may contain multiple controllers](images/syscfg_multi_ctrl.svg)

![Environments may contain multiple populations](images/syscfg_multi_pop.svg)

![Management programs may contain multiple environments](images/syscfg_multi_env.svg)


## Filesystem Layout ##

The outputs of the NPC Maker are organized using the following directory structure:  
`project_directory/run_directory/population_directory/algorithm_directory/individual.json`  

### project_directory ###

This folder is for each project or experiment to store its custom files in.  
The NPC Maker doesn't look here, but users will almost certainly have stuff here.

### run_directory ###

This folder is for the computer generated outputs of the NPC Maker. These files
are kept separate from the human created files so that they can be regenerated
without accidentally destroying the human labors.

### population_directory ###

This folder is for each population. These directories are named after the
population's name.

### algorithm_directory ###

Each program that interacts with the NPC Maker should create new directories as
necessary to avoid cluttering up the shared directories with a large number of
auto-generated files.


## Utilities ##

Although the NPC Maker is built on the foundation of its interfaces, it provides
many tools and utilities for accomplishing common tasks. This section gives an
overview of the built-in tools.

### npc_maker.env.Environment.run ###

### npc_maker.env.SoloAPI ###

### npc_maker.evo.Recorder ###

### npc_maker.evo.Replayer ###

### npc_maker.evo.Evolution ###

#### population types ####

#### mate selection algorithms ####

#### crossover algorithms ####

### npc_maker.env.Remote ###

### npc_maker.ctrl.Remote ###

