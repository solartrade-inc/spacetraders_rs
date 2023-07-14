# Automation + Control Api

An automaton (/ɔːˈtɒmətən/; pl automata or automatons) is a relatively self-operating machine,
or control mechanism designed to automatically follow a sequence of operations, or respond to
predetermined instructions.

## Automaton
We consider the world 'state' as a set of variables that describe the current state of the world.
The automaton is a function that takes the current state of the world and returns an action to
perform. The action is a set of variables that describe the action to perform. The action is
performed on the world and the world is updated. The automaton is then called again with the new
state of the world.

## Control
At any point in time, the automaton can be re-configured with a new set of instructions. These take
effect immediately and the automaton will start following the new instructions, when it computes its
next action. This is analogous to updating the function that the automaton is.


## Automaton Details
State: 
- location of all ships, waypoints, and systems
- ship modules and cargo
- ship travel info, cooldowns, and fuel
- known market prices
- previously stored state information, scoped to the ship or planet or sytem

Action:
- api call(s) to make

## Single control flow, as opposed to multiple control flows
Treating each ship as just one piece of many in a larger system, with a single control flow, is
the chosen design because it's more flexible and easier to reason about when it comes to inter-ship
coordination. It's also more in line with the game's design, where each ship is a single entity
that can be controlled by a single player. Running a single loop has to be simpler than running
multiple loops and coordinating between them.

The only downside I can see is whether it is fast enough to run all the ships in a single loop. But at
3 api calls per second, it should be fine. We probably do want to think about the next action before
the api call returns though. Which means we may need a lock system of some sort, since you cannot
make multiple api calls at the same time for the same ship.

## Influencing the control flow
The control flow is influenced by the state of the world. We can provide a http endpoint that
allows the operator to update certain configuration parameters that influence the control flow.

