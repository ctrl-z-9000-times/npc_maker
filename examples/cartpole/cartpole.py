#!/usr/bin/python
"""
Inverted Pendulum Environment

The goal of the controller in this environment is to balance an upright pole on
a moving cart. This is on of the simplest and easiest controls problems.
For added difficulty try it with wind blowing on the pole, use the settings:
-   wind [0-10]
-   gusts [0-10]

The cartpole problem is noteworthy because its very easy to solve and its
solution requires a closed loop controller. This environment is useful for
researching and developing techniques related to closed loop controllers.

For implemention details see:

    Evolving Neural Networks through Augmenting Topologies
    Stanley and Miikkulainen, 2002
    https://doi.org/10.1162/106365602320169811

"""
from npc_maker import env as env_api
from npc_maker.ctrl import Controller
from pathlib import Path
from wind import wind
import collections
import json
import math
import mujoco
import mujoco.viewer
import random
import time

class State:
    START = 0
    STOP  = 1 # Run until done, and then don't request another individual.
    PAUSE = 2
    QUIT  = 3

idle_fps = 30
population = "cartpole"

position_range  = 2.4
velocity_range  = 1.5
angle_range     = math.radians(36)
ang_vel_range   = math.radians(115)

def sweep_initial_states(trial_num):
    """
    Perform a systematic sweep of the initial state space.
    """
    percents      = [0, -0.25, 0.90, 0.25, -.90]
    num_percents  = len(percents)
    init_position = 0
    init_velocity = 0
    init_angle    = 0
    init_ang_vel  = 0
    if trial_num > num_percents:
        init_position = position_range * percents[trial_num % num_percents]
        trial_num = int(trial_num / num_percents)
    if trial_num > num_percents:
        init_angle = angle_range * percents[trial_num % num_percents]
        trial_num = int(trial_num / num_percents)
    if trial_num > num_percents:
        init_ang_vel = ang_vel_range * percents[trial_num % num_percents]
        trial_num = int(trial_num / num_percents)
    if trial_num > num_percents:
        init_velocity = velocity_range * percents[trial_num % num_percents]
        trial_num = int(trial_num / num_percents)
    # Always add a little bit of noise.
    noise          = 0.001
    position_noise = position_range * math.radians(random.uniform(-noise, noise))
    velocity_noise = velocity_range * math.radians(random.uniform(-noise, noise))
    angle_noise    = angle_range    * math.radians(random.uniform(-noise, noise))
    ang_vel_noise  = ang_vel_range  * math.radians(random.uniform(-noise, noise))
    return [init_position + position_noise,
            init_velocity + velocity_noise,
            init_angle    + position_noise,
            init_ang_vel  + velocity_noise]

class CartpoleEnvironment(env_api.SoloAPI):
    def __init__(self, spec, mode, **settings):
        self.spec = spec
        self.mode = mode
        # Initialize the MuJoCo simulation.
        mjcf_file  = str(spec["spec"].with_suffix(".mjcf"))
        self.model = mujoco.MjModel.from_xml_path(mjcf_file)
        self.data  = mujoco.MjData(self.model)
        if self.mode == "graphical":
            self.viewer = mujoco.viewer.launch_passive(self.model, self.data,
                                        show_left_ui=False, show_right_ui=False)
            with self.viewer.lock():
                # This does not appear to work.
                self.viewer.cam.fixedcamid = self.data.camera("fixed").id
        else:
            self.viewer = None
        # Initialize the environment's state.
        self.num_poles  = spec["num_poles"]
        self.state      = State.STOP
        self.dt         = self.model.opt.timestep
        self.duration   = settings["duration"]
        self.max_steps  = math.ceil(self.duration / self.dt)
        self.init_angle = settings["angle"]
        self.sweep      = settings["sweep"]
        self.num_trials = settings["trials"]
        if self.sweep: self.num_trials = 625
        self.wind_speed = settings["wind"]
        self.gust_speed = settings["gust"]
        self.reset_individual()

    def is_running(self):
        return ((self.state == State.START or
                 self.state == State.STOP) and
                 self.individual is not None)

    def reset_individual(self):
        """
        Initialize the empty fields for an individual's data.
        """
        self.individual   = None
        self.ctrl         = None
        self.scores       = [] # Score for each trial.
        self.survived     = [] # How many timesteps it survive in each trial.
        self.oscillations = collections.deque()

    def birth(self, name, genome, population, controller, **extra):
        self.reset_individual()
        self.individual = name # UUID string.
        self.ctrl = Controller(self.spec["spec"], population, controller)
        self.ctrl.new(json.dumps(genome))
        self.start_trial() # Reset the environment for the new individual.

    def start_trial(self):
        """
        Reset the simulation and control system. Prepare for a new trial.
        """
        self.wind = wind(self.duration, self.dt, self.wind_speed, self.gust_speed)
        mujoco.mj_resetData(self.model, self.data)
        if self.ctrl is not None:
            self.ctrl.reset()
        self.num_steps = 0
        # Initialize the cart & pole positions.
        if self.sweep:
            trial_num  = len(self.scores)
            p, v, a, q = sweep_initial_states(trial_num)
            self.data.joint("slider").qpos[0]  = p
            self.data.joint("slider").qvel[0]  = v
            self.data.joint("hinge_1").qpos[0] = a
            self.data.joint("hinge_1").qvel[0] = q
        else:
            init_angle = random.uniform(-self.init_angle, self.init_angle)
            self.data.joint("slider").qpos[0]  = 0.0
            self.data.joint("slider").qvel[0]  = 0.0
            self.data.joint("hinge_1").qpos[0] = math.radians(init_angle)
            self.data.joint("hinge_1").qvel[0] = 0.0

    def update_wind(self):
        """
        Returns the current wind speed.
        """
        if self.wind:
            wind = self.wind.pop()
        else:
            wind = 0.0
        self.model.opt.wind[0] = wind
        return wind

    def advance_controller(self, position, velocity, angle_1, angle_2, wind):
        """
        Run the control system. Transfer sensory inputs and motor outputs
        between the simulation and the controller.
        """
        if not self.ctrl.is_alive():
            self.quit()
        bias_gin     = 0
        angle_1_gin  = 1
        angle_2_gin  = 2
        position_gin = 3
        wind_gin     = 4
        motor_gin    = 5
        self.ctrl.set_input(bias_gin,     1.0)
        self.ctrl.set_input(angle_1_gin,  angle_1)
        self.ctrl.set_input(position_gin, position)
        self.ctrl.set_input(wind_gin,     wind)
        if self.num_poles >= 2:
            self.ctrl.set_input(angle_2_gin, angle_2)
        # 
        self.ctrl.advance(self.dt)
        # Write the motor outputs into the mujoco simulation.
        output = float(self.ctrl.get_outputs(motor_gin))
        # output = 2.0 * (output - 0.5)
        self.data.actuator("slide").ctrl = output

    def advance_environment(self):
        wind = self.update_wind()
        # Read the cart's sensor data.
        position    = self.data.joint("slider").qpos[0]
        velocity    = self.data.joint("slider").qvel[0]
        angle_1     = self.data.joint("hinge_1").qpos[0]
        ang_vel     = self.data.joint("hinge_1").qvel[0]
        if self.num_poles < 2:
            angle_2   = 0.0
        else:
            angle_2 = self.data.joint("hinge_2").qpos[0]
        # Scale the sensory inputs into the range [-1, +1]
        position /= position_range
        velocity /= velocity_range
        angle_1  /= angle_range
        angle_2  /= angle_range
        ang_vel  /= ang_vel_range
        # Run the control system.
        if self.ctrl is not None:
            self.advance_controller(position, velocity, angle_1, angle_2, wind)
        # Measure the magnitude of the oscillations.
        self.oscillations.append(
            abs(position) +
            abs(velocity) +
            abs(angle_1) +
            abs(ang_vel))
        while len(self.oscillations) > 100:
            self.oscillations.popleft()
        # Advance the mujoco simulation.
        mujoco.mj_step(self.model, self.data)
        self.num_steps += 1
        # Check if the trial is over.
        fallen_pole_1     = abs(angle_1) > 1
        fallen_pole_2     = abs(angle_2) > 1
        runaway_cart      = abs(position) >= 1
        timelimit_reached = self.num_steps >= self.max_steps
        survived          = not (fallen_pole_1 or fallen_pole_2 or runaway_cart)
        if not survived or timelimit_reached:
            # Determine how many timesteps the controller survived.
            if not survived:
                self.num_steps -= 1 # Ignore the current timestep if it died.
            self.survived.append(self.num_steps)
            # Calculate the score for this trial.
            time_score = self.num_steps / self.max_steps
            if len(self.oscillations) < 100:
                oscillation_score = 0
            else:
                oscillation_score = 0.75 / sum(self.oscillations)
            score = 0.1 * time_score + 0.9 * oscillation_score
            self.scores.append(score)
            # Either prepare this individual for its next trial
            # or report its final score.
            if len(self.scores) >= self.num_trials:
                self.report_death()
            else:
                self.start_trial()

    def report_death(self):
        """
        Calculate the current individual's final score.
        Reports it back to the management program.
        """
        # Report the average score.
        if self.scores:
            env_api.score(self.individual, sum(self.scores) / len(self.scores))
        # Report the fraction of the trials that were successful.
        if self.num_trials:
            successful_trials = sum(num_steps >= self.max_steps
                                    for num_steps in self.survived)
            env_api.info(self.individual,
                {"generalization": successful_trials / self.num_trials})
        env_api.death(self.individual)
        self.reset_individual()
        # Request a new individual.
        if self.state == State.STOP:
            env_api.ack("Stop")
        else:
            env_api.new(population)

    def advance(self, controller):
        """
        Run the environment forward one step.
        """
        self.ctrl = controller
        if self.is_running():
            self.advance_environment()

        if self.mode == "graphical":
            self.update_graphics()
        elif not self.is_running():
            time.sleep(1 / idle_fps) # Don't excessively busy loop.

    def update_graphics(self):
        """ Update the graphical output. """
        if self.viewer is not None and self.viewer.is_running():
            # Sleep until the start of the next frame.
            if hasattr(self, "_last_frame_timestamp"):
                if self.is_running():
                    elapsed_time = time.monotonic() - self._last_frame_timestamp
                    frame_time   = self.dt - elapsed_time
                else:
                    # If not running then cap the FPS.
                    frame_time = 1 / idle_fps
                time.sleep(max(0, frame_time))
            # 
            self.viewer.sync()
            self._last_frame_timestamp = time.monotonic()

    def quit(self):
        """ Cleanup and exit. """
        if self.viewer is not None:
            self.viewer.close()
        self.reset_individual()
        self.model  = None
        self.data   = None
        self.viewer = None

if __name__ == "__main__":
    CartpoleEnvironment.main()
