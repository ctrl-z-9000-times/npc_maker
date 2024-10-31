from npc_maker.evo import Individual

def test_name():
    indiv1 = Individual("test_env", "test_pop", "test_ctrl", "test_genome")
    indiv2 = Individual("test_env", "test_pop", "test_ctrl", "test_genome")
    assert indiv1.get_name() == indiv1.get_name()
    assert indiv1.get_name() != indiv2.get_name()

def test_save_load():
    indiv1 = Individual("test_env", "test_pop", "test_ctrl", "test_genome")
    print(vars(indiv1))
    path = indiv1.save("./")
    try:
        print(open(path, "rt").read())
        indiv2 = Individual.load(path, "test_ctrl")
        print(vars(indiv2))
        assert vars(indiv1) == vars(indiv2)
    finally:
        path.unlink()
