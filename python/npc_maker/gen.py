"""
Genetic Tools, for manipulating genomes.

Genome format: unlike the other APIs, this module imposes a specific format on
the genomes which it handles. In addition to being JSON-encodable objects,
these tools expect genomes to be arrays of chromosomes, where each chromosome
is a dictionary with the entry: {"name": global-innovation-number}.
"""

import random

def _genome_array_to_dict(genome_array, genome_dict):
    """
    Transform the genome from a list of chromosomes,
    into a dictionary of lists of chromosomes, indexed by GIN.
    """
    for chromosome in genome_array:
        gin = chromosome["name"]
        if gin not in genome_dict:
            genome_dict[gin] = [chromosome]
        else:
            genome_dict[gin].append(chromosome)
    return genome_dict

def _genome_select(genome, num_parents=0):
    """
    Transform the genome from a dictionary of lists of chromosomes,
    to a list of chromosomes, by randomly selecting one chromosome
    from each list.

    Argument num_parents allows "inheriting" missing chromosomes. If num_parents
             is zero (the default) then a chromosome will always be inherited.
    """
    missing = []
    for gin, chromosomes in genome.items():
        if len(chromosomes) < num_parents:
            if random.random() > len(chromosomes) / num_parents:
                missing.append(gin)
                continue
        genome[gin] = random.choice(chromosomes)
    for gin in missing:
        del genome[gin]
    return list(genome.values())

def monoploid_crossover(parents) -> 'genome':
    """
    Combine parent genomes to create a new child genome.

    Argument parents is a list of genomes.

    Monoploid genomes have one instance of each chromosome, as identified by
    global innovation number (GIN). One of the parents is randomly selected to
    inherit each GIN from. If the selected parent has multiple chromosomes with
    the GIN in question, then one of the chromosomes is selected at random.
    If the selected parent does not have any chromosomes with that GIN, then the
    child will not inherit a chromosome with that GIN.
    """
    parents = [p.get_genome() for p in parents]
    # Transform the parent genomes into "gamete" genomes for the child to
    # inherit from. Gametes have no duplicate GINs.
    gametes = [_genome_select(_genome_array_to_dict(genome, {})) for genome in parents]
    # Merge the gametes into one big genome.
    child = {}
    for gamete in gametes:
        _genome_array_to_dict(gamete, child)
    # 
    num_parents = len(gametes)
    return _genome_select(child, num_parents)

def polyploid_crossover(parents) -> 'genome':
    """
    Combine parent genomes to create a new child genome.

    Argument parents is a list of genomes.

    Polyploid genomes have one complete set of chromosomes per parent.
    Chromosomes are identified by their global innovation number (GIN).
    Polyploid genomes may have multiple chromosomes with the same GIN.
    To reproduce: each parent creates a "gamete" genome which has a single
    complete copy of the genome. Gametes have no duplicate chromosomes.
    The child genome is the concatenation of its parent's gametes.
    """
    child = []
    for genome in parents:
        gamete = _genome_array_to_dict(genome, {})
        num_parents = max(len(chromosomes) for chromosomes in gamete.values())
        gamete = _genome_select(gamete, num_parents)
        child.extend(gamete)
    return child
