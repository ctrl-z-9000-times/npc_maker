import npc_maker.evo

class NeatGenome(npc_maker.evo.Genome):
    def __init__(self):
        self.nodes = []
        self.edges = []

    def parameters(self):
        1/0

    def mate(self, other):
        1/0

    def distance(self, other):
        1/0

class Node:
    __slots__ = ('name', 'slope', 'midpoint', 'disabled')

class Edge:
    __slots__ = ('name', 'presynapse', 'postsynapse', 'weight', 'disabled')
