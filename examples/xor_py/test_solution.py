"""
Solve the XOR environment using the NN controller.

This tests all combinations of programming languages.
"""

import npc_maker.env
from pathlib import Path

repo = Path(__file__).parent.parent.parent

environments = [
    repo.joinpath("examples/xor/xor.env"),
    # repo.joinpath("examples/xor/xor_rust.env"),
]

controllers = [
    repo.joinpath("examples/nn/nn.py"),
    # repo.joinpath("examples/nn/target/debug/logistic"),
]

solution = [
    {"name": 0, "type": "Node", "slope": 2.0, "midpoint":  0.5},
    {"name": 1, "type": "Node", "slope": 2.0, "midpoint":  0.5},
    {"name": 2, "type": "Node", "slope": 2.0, "midpoint":  0.5},
    {"name": 3, "type": "Node", "slope": 2.0, "midpoint":  2.0},
    {"name": 6, "type": "Edge", "presyn": 0, "postsyn": 2, "weight": 1.0},
    {"name": 7, "type": "Edge", "presyn": 1, "postsyn": 2, "weight": 1.0},
    {"name": 8, "type": "Edge", "presyn": 3, "postsyn": 2, "weight": -4.0},
    {"name": 10, "type": "Edge", "presyn": 0, "postsyn": 3, "weight": 1.0},
    {"name": 11, "type": "Edge", "presyn": 1, "postsyn": 3, "weight": 1.0}]

def test_solution():
    for env_path in environments:
        for ctrl_cmd in controllers:
            print("Testing:", env_path, ctrl_cmd)
            individual  = {"controller": ctrl_cmd, "genome": solution}
            results     = npc_maker.env.Environment.run({"xor": [individual]}, env_path)
            score       = float(results["xor"][0].get_score())
            assert score >= 15.0

if __name__ == "__main__":
    test_solution()
