# OpenSolver

Open source postflop solver for Texas Hold'em Poker written in Rust with UPI (Universal Poker Interface) compatibility. Algorithm used is Discounted CFR (DCFR). First project used to learn Rust. 

## Performance
Compared to commercial solvers, it is about 2x slower for rainbow flops (three distinct suits) and even worse for two tone and monotone flops, due to isomorphism not being correctly implemented. 

## TODOs

- Performance
- - Make it more memory efficient: f64 -> f32, implement compression technique proposed in https://poker.cs.ualberta.ca/publications/2015-ijcai-cfrplus.pdf
- - Explore other algorithms, e.g. https://realworld-sdm.github.io/paper/27.pdf
- - Implement isomorphism (lossless abstraction)
- General
- - Add more UPI commands
- - Better error handling
- - Tests

## Resources
[1] DCFR algorithm: https://arxiv.org/pdf/1809.04040.pdf
[2] c++ Poker Solver: https://github.com/Fossana/cplusplus-cfr-poker-solver Current rust code base was heavily based on this code
