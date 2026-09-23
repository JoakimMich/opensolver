# OpenSolver

Open source postflop solver for Texas Hold'em Poker written in Rust with UPI (Universal Poker Interface) compatibility. Algorithm used is Discounted CFR (DCFR). First project used to learn Rust. 

## Performance
Solving speed is now on par/better than commercial solver thanks to Claude optimizations.
Turn and river cards that are isomorphic (suits that the board, the earlier streets and both ranges treat identically) are solved once and derived for the other suits. Toggle with `set_isomorphism <flop trees> <turn trees>` (default `1 0`: on for trees starting on the flop, off for trees starting on the turn, as in PioSolver).

## TODOs

- More UPI Commands
- GUI

## Resources
[1] DCFR algorithm: https://arxiv.org/pdf/1809.04040.pdf

[2] c++ Poker Solver: https://github.com/Fossana/cplusplus-cfr-poker-solver Current rust code base was heavily based on this code
