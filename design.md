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
