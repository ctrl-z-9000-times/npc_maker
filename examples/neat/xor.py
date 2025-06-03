"""
Solve the XOR environment using the NEAT algorithm.
"""

import npc_maker.env
import npc_maker.evo
import neat
from pathlib import Path

repo = Path(__file__).parent.parent.parent
environment = repo.joinpath("examples/xor_py/xor.env"),
controller  = repo.joinpath("examples/nn_py/nn.py"),

def test_neat():
    # Make the seed.
    genome = neat.NeatGenome()
    seed = npc_maker.evo.Individual(genome, controller=controller)
    evo = npc_maker.evo.Evolution(seed, population_size, speciation_distance, species_distribution, mate_selection)
    env_pool = [npc_maker.env.Environment(environment) for _ in range(1)]
    for env in env_pool: env.start()

    # Run the evolutionary algorithm.
    while evo.get_generations() < 100:
        for env in env_pool:
            env.poll()
            time.sleep(0)

    # Check results.
    1/0

if __name__ == "__main__":
    test_neat()
