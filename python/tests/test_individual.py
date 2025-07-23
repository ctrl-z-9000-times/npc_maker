from npc_maker.evo import Individual, Genome

class TestGenome(Genome):
    def __init__(self, data):
        self.data = data

def test_name():
    indiv1 = Individual(b"test_genome")
    indiv2 = Individual(b"test_genome")
    assert indiv1.get_name() == indiv1.get_name()
    assert indiv1.get_name() != indiv2.get_name()

def test_save_load():
    indiv1 = Individual(
        controller="test_ctrl",
        genome= TestGenome(b"test_genome"),
        ascension=777,
        info={"test": "hello world"},
        foo="bar")
    print(vars(indiv1))
    path = indiv1.save("./")
    try:
        print(open(path, "rb").read())
        indiv2 = Individual.load(TestGenome, path)

        indiv1._genome = None
        assert vars(indiv1) == vars(indiv2)
        assert indiv1.get_genome().data == indiv2.get_genome().data
    finally:
        path.unlink()
