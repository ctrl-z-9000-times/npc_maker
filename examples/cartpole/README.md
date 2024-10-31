# Example Environment: Cartpole

The goal of cart-pole challenge is to balance an inverted pendulum.
This is a classic problem in control theory and has been studied extensively.
An inverted pendulum is a thin rigid rod balanced vertically on its end.
It is unstable. It will fall over because its center of mass is far above its
point of contact with the ground. The rod is placed on a moving platform, and
by moving the platform underneath the rod we can keep the rod balanced.
This challenge is akin to balancing an object on your fingertip. For simplicity
the entire system is constrained to two dimensions and the platform is on
horizontal rails.

<img alt="Schematic Diagram" src="https://upload.wikimedia.org/wikipedia/commons/0/00/Cart-pendulum.svg" width="300">

Once we've mastered the basic inverted pendulum, we will modify the task to
demonstrate different capabilities. First control the position of the cart in
addition to balancing the pole, then add additional poles, and finally swing
the pendulum upright from a hanging position. We can also make these challenges
easier or harder by changing the physical properties of the scenario. For
example: longer poles have further to fall, which gives us more time to react
and makes the system easier to control. On the other hand strong gusts of wind
blowing the pole in random directions make the system more difficult to control.
