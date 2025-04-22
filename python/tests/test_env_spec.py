from pathlib import Path
from npc_maker import env
from pprint import pprint

def test_env_spec():
    # Parse all env-spec's in the examples directory.
    repo_dir = Path(__file__).parent.parent.parent
    examples_dir = repo_dir.joinpath("examples")
    for env_spec in examples_dir.glob("*/*.env"):
        print(f"Loading Environment Specification: \"{env_spec}\" ...")
        pprint(env.Specification(env_spec))
