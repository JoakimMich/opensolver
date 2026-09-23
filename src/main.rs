use crate::upi::*;

mod postfloptree;
mod range;
mod cfr;
mod hand_range;
mod best_response;
mod trainer;
mod isomorphism;
mod upi;
mod cards;
#[cfg(test)]
mod ev_tests;



#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let mut cli_session = CliSession::new();
    cli_session.start();
}
