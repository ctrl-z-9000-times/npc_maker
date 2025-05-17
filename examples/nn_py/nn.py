#!/usr/bin/python

"""
Example controller - artificial neural network

This file demonstrates how to implement a control system for the NPC Maker.
"""

from npc_maker import ctrl
import json
import math

def logistic(value, slope=1, midpoint=0):
    # The magic number 4.0 scales the maximum slope of the curve to 1.0
    x = 4.0 * slope * (value - midpoint)
    try:
        e = math.exp(-x)
    except OverflowError:
        e = math.inf
    return 1.0 / (1.0 + e)

class NN(ctrl.API):
    def new(self, environment, population, genotype):
        self.names      = {}
        self.states     = []
        self.slopes     = []
        self.midpoints  = []
        self.edges      = []
        # 
        for entity in genotype:
            if entity["type"] == "Node":
                gin = int(entity["name"])
                idx = len(self.names)
                self.names[gin] = idx
                self.states.append(0.0)
                self.slopes.append(float(entity["slope"]))
                self.midpoints.append(float(entity["midpoint"]))
        # 
        for entity in genotype:
            if entity["type"] == "Edge":
                presyn  = int(entity["presyn"])
                postsyn = int(entity["postsyn"])
                presyn  = self.names[presyn]
                postsyn = self.names[postsyn]
                weight  = float(entity["weight"])
                self.edges.append((presyn, postsyn, weight))

    def reset(self):
        for idx in range(len(self.states)):
            self.states[idx] = 0.0

    def advance(self, dt):
        next_states = [0.0 for _ in range(len(self.states))]
        for (presyn, postsyn, weight) in self.edges:
            next_states[postsyn] += weight * self.states[presyn]
        for i in range(len(self.states)):
            next_states[i] = logistic(next_states[i], self.slopes[i], self.midpoints[i])
        self.states = next_states

    def set_input(self, gin, value):
        gin   = int(gin)
        value = float(value)
        index = self.names[gin]
        self.states[index] = value

    def get_output(self, gin):
        gin   = int(gin)
        index = self.names[gin]
        return self.states[index]

if __name__ == "__main__":
    ctrl.main(NN())
