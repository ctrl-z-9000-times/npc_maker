"""
Generate synthetic wind speed data for mujoco simulations
"""

import math
import random
import numpy

def wind(period, dt, wind_speed, gust_speed):
    """
    Argument wind_speed is the maximum sustained wind speed.
    Argument gust_speed is the maximum transient gust speed.

    Returns a list of samples spanning the given period at regular intervals of dt.
    """
    wind_speed      = float(wind_speed)
    gust_speed      = float(gust_speed)
    dt              = float(dt)
    wind_speed      = random.uniform(-wind_speed, wind_speed)
    gust_speed      = random.uniform(-gust_speed, gust_speed)
    samples         = math.ceil(period / dt)
    elapsed         = numpy.linspace(0, period, samples)
    gust_tau_1      = math.tau / random.uniform(5, 30)
    gust_tau_2      = math.tau / random.uniform(30, 120)
    gust_phase_1    = random.uniform(0, math.tau)
    gust_phase_2    = random.uniform(0, math.tau)
    gust_ratio      = random.uniform(.1, .9)
    gust_1          = numpy.sin(elapsed * gust_tau_1 + gust_phase_1)
    gust_2          = numpy.sin(elapsed * gust_tau_2 + gust_phase_2)
    gust_signal     = (gust_ratio * gust_1 + (1.0 - gust_ratio) * gust_2)
    timeseries      = wind_speed + gust_speed * gust_signal
    return list(timeseries)

if __name__ == '__main__':
    import matplotlib.pyplot as plt
    import argparse
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument('--duration', type=float, default=60)
    args.add_argument('--dt',   type=float, default=0.001)
    args.add_argument('--wind', type=float, default=2)
    args.add_argument('--gust', type=float, default=2)
    args = args.parse_args()
    wind_speed = wind(args.duration, args.dt, args.wind, args.gust)
    timestamps = numpy.linspace(0, args.duration, len(wind_speed))
    plt.figure("wind.py")
    plt.title('Sample of Wind Speed Function')
    plt.plot(timestamps, wind_speed)
    plt.ylabel('Wind Speed (meters/second)')
    plt.xlabel('Elapsed Time (seconds)')
    # Always show zero-wind speed on the graph.
    plt.axhline(0, color='k', linewidth=0.75)
    plt.show()
