from pathlib import Path
import mate_selection
import npc_maker.env
import npc_maker.evo
import time

ctrl_cmd = Path(__file__).parent.joinpath("pid_ctrl.py")
env_spec = Path(__file__).parent.joinpath("cartpole.env")

solution = {"controller": ctrl_cmd, "genome": [
    {
        "name": 1,
        "kp": 0.5,
        "ki": 0.7,
        "kd": 0.04,
    }
]}

def mutate(genome):
    for chromosome in genome:
        chromosome["kp"]
        chromosome["ki"]
        chromosome["kd"]
    return genome

def demo():
    """ Run the solution in a loop forever. """
    import itertools
    npc_maker.env.Environment.run(
            {"cartpole": [itertools.cycle([solution])]}, env_spec)

def test_solution():
    results = npc_maker.env.Environment.run({"cartpole": [solution]}, env_spec)
    assert float(results["cartpole"][0].score) >= 20

def test_evolution():
    seed = solution["genome"]
    service = npc_maker.evo.Evolution(ctrl_cmd, seed, mutate,
        population_size=200,
        select=mate_selection.Percentile(.80))

    env = npc_maker.env.Environment({"cartpole": service}, env_spec, 'headless')
    env.start()

    while True:
        env.poll()
        time.sleep(.1)

    env.quit()

if __name__ == "__main__":
    # test_solution()
    test_evolution()
    # demo()
