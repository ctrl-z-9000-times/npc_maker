#!/usr/bin/python

"""
Plot the logistic function.
"""

from math import exp

def logistic(value, slope=1, halfway=0):
    # The magic number 4.0 scales the maximum slope of the curve to 1.0
    x = 4.0 * slope * (value - halfway)
    return 1.0 / (1.0 + exp(-x) )

if __name__ == '__main__':
    import numpy as np
    import matplotlib.pyplot as plt
    import argparse
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument('--slope', type=float, default=1)
    args.add_argument('--halfway', type=float, default=0)
    args = args.parse_args()
    num_pts  = 100
    radius   = 1 / args.slope
    low      = args.halfway - radius
    high     = args.halfway + radius
    x_coords = np.linspace( low, high, num_pts )
    plt.figure('Logistic Function')
    plt.plot(x_coords, [logistic(x, args.slope, args.halfway) for x in x_coords])
    plt.ylim(0, 1)
    plt.show()
