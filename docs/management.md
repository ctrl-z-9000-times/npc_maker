# The Management Program #

This chapter describes how to set up and run the NPC Maker framework. The
framework consists of many pieces which need to be gathered up, assembled
correctly, and set in motion. The management program is responsible for
creating and controlling instances of the environment and evolution services.

See the [examples](../examples/) directory for practical guidance on using the NPC Maker's APIs.


## Built-in Services ##

[Discuss the filesystem layout]

### Recorder and Replayer ###

[TODO: Briefly describe these two utility classes]

### Evolution ###

[discuss mate-selection, population types, crossover types, etc]

### Tools to make envs out of simple functions ###
todo


## Example System Configurations ##

There are many possible ways for you, the user, to set up your computer systems.  
Here are a few examples:

![Minimal Configuration](images/syscfg_minimal.svg)

![Environments may contain multiple controllers](images/syscfg_multi_ctrl.svg)

![Environments may contain multiple populations](images/syscfg_multi_pop.svg)

![Management programs may contain multiple environments](images/syscfg_multi_env.svg)

