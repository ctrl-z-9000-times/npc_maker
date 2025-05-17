
from npc_maker.ctrl import Controller
from pathlib import Path
import json

ctrl_prog = Path(__file__).parent.joinpath("nn.py")

genome = json.dumps([
    {
        "name": 0,
        "type": "Node",
        "slope": 10,
        "midpoint": 0.5
    },
    {
        "name": 1,
        "type": "Node",
        "slope": 10,
        "midpoint": 0.5
    },
    {
        "name": 2,
        "type": "Edge",
        "presyn": 0,
        "postsyn": 1,
        "weight": 2
    }
])

def test_nn():
    x = Controller("my_env", "my_pop", [ctrl_prog])
    x.new(genome)
    x.set_input(0, 42)
    x.advance(0.01)
    assert x.is_alive()
    assert float(x.get_outputs(0)) < .001
    assert float(x.get_outputs(1)) > .999
    assert float(x.get_outputs(1)) > .999
    x.reset()
    x.advance(0.01)
    assert float(x.get_outputs(1)) < .001
    x.quit()
    del x

if __name__ == "__main__":
    print(ctrl_prog)
    test_nn()
