"""
Solve the XOR environment using the NN controller.

This tests all combinations of programming languages.
"""

from npc_maker.env import Environment
from npc_maker.evo import Evolution

environments = [
    "examples/xor/xor.env",
    # "examples/xor/xor_rust.env",
]

controllers = [
    "examples/nn/nn.py",
    # "examples/nn/target/debug/logistic",
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

class Dispatcher(Evolution):
    def __init__(self, ctrl_cmd):
        self.ctrl_cmd = ctrl_cmd
    def controller(self):
        return self.ctrl_cmd
    def birth(self, parents):
        return solution
    def death(self, indiv):
        score = float(indiv.get_score())
        assert score >= 15.0
        raise StopIteration

for env_path in environments:
    for ctrl_cmd in controllers:
        print("Testing:", env_path, ctrl_cmd)
        disp  = Dispatcher(ctrl_cmd)
        env   = Environment({"xor": disp}, env_path)
        env.start()
        while True:
            try:
                env.poll()
            except StopIteration:
                break
