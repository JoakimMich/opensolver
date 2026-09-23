// EV regression tests on spots with known analytic solutions.
//
// All spots are river "AA/QQ vs KK" toy games on 2c2d2s3c3d: every hand plays a full house,
// AA beats KK beats QQ. OOP holds a polarized range (AA value, QQ bluffs), IP holds a pure
// bluff-catcher (KK). OOP may check or bet B into pot P; IP can only check behind or call/fold.
//
// Equilibrium: OOP bluffs so IP is indifferent (bluffs/value = B/(P+B)), IP calls
// P/(P+B) so bluffing QQ is indifferent to checking (both EV 0). EVs are in chips won from
// the pot, so OOP EV + IP EV = P at equilibrium.

use crate::best_response::*;
use crate::hand_range::*;
use crate::range::*;
use crate::trainer::*;
use crate::upi::*;

const BOARD: &str = "2c2d2s3c3d";
const POT: u32 = 100;
const ITERATIONS: u64 = 2000;
const TOLERANCE: f64 = 0.1;

// Solves the spot and returns (OOP EV, IP EV), each being that player's best-response value
// against the opponent's average strategy. Both bracket the true game value, so they match it
// once the solve has converged.
fn solve(oop_range: &str, ip_range: &str, stack: u32, lines: Vec<Vec<u32>>) -> (f64, f64) {
    let range_manager = RangeManager::new(HandRange::from_string(oop_range.to_string()), HandRange::from_string(ip_range.to_string()), BOARD.to_string());
    let mut trainer = Trainer::new(range_manager, lines, stack, POT);
    trainer.train(&Accuracy::Chips(0.0), TrainFinish::Iterations(ITERATIONS));
    solver_evs(&trainer)
}

fn solver_evs(trainer: &Trainer) -> (f64, f64) {
    let mut best_response = BestResponse::new(&trainer.range_manager);
    best_response.set_relative_probablities(true);
    best_response.set_relative_probablities(false);
    let half_pot = trainer.root.pot_size as f64 / 2.0;
    let oop_ev = best_response.get_best_response_ev(true, &trainer.root) / 2.0 + half_pot;
    let ip_ev = best_response.get_best_response_ev(false, &trainer.root) / 2.0 + half_pot;
    (oop_ev, ip_ev)
}

fn assert_ev(oop_range: &str, ip_range: &str, stack: u32, lines: Vec<Vec<u32>>, expected_oop_ev: f64) {
    let (oop_ev, ip_ev) = solve(oop_range, ip_range, stack, lines);
    let expected_ip_ev = POT as f64 - expected_oop_ev;
    assert!((oop_ev - expected_oop_ev).abs() < TOLERANCE, "OOP EV {} != expected {}", oop_ev, expected_oop_ev);
    assert!((ip_ev - expected_ip_ev).abs() < TOLERANCE, "IP EV {} != expected {}", ip_ev, expected_ip_ev);
}

#[test]
fn checkdown_only() {
    // No betting: AA wins the pot, QQ loses it.
    assert_ev("AA,QQ", "KK", POT, vec![vec![0, 0]], 50.0);
}

#[test]
fn pot_sized_bet() {
    // B = P: QQ bluffs 1/2, KK calls 1/2. AA wins 100 + 50, QQ wins 0.
    assert_ev("AA,QQ", "KK", 100, vec![vec![0, 0], vec![100]], 75.0);
}

#[test]
fn half_pot_bet() {
    // B = P/2: QQ bluffs 1/3, KK calls 2/3. AA wins 100 + 2/3 * 50, QQ wins 0.
    assert_ev("AA,QQ", "KK", 50, vec![vec![0, 0], vec![50]], 200.0 / 3.0);
}

#[test]
fn single_bluff_catcher_combo() {
    // IP holding a single combo must not change the EVs (no card removal between the ranges).
    assert_ev("AA,QQ", "KcKd", 100, vec![vec![0, 0], vec![100]], 75.0);
}

#[test]
fn bluff_heavy_range() {
    // AA at half weight: 3 value combos vs 6 bluffs. QQ bluffs 1/4 of the time, KK calls 1/2.
    // OOP EV = (3 * 150 + 6 * 0) / 9.
    assert_ev("AA@50,QQ", "KK", 100, vec![vec![0, 0], vec![100]], 50.0);
}

#[test]
fn value_heavy_range() {
    // QQ at quarter weight: too few bluffs to make KK indifferent, so OOP bets everything and
    // KK always folds. OOP always wins the pot.
    assert_ev("AA,QQ@25", "KK", 100, vec![vec![0, 0], vec![100]], 100.0);
}

// Full flop solve (single-raised pot, QsJh2h, 180 pot, 910 stacks) replayed from a PIO UPI script.
// Expected EVs are PIO's for the same tree. Takes about a minute in release, so it is ignored by
// default: run with `cargo test --release -- --ignored`.
#[test]
#[ignore]
fn qsjh2h_srp_matches_pio() {
    let mut session = CliSession::new();
    for line in include_str!("../tests/fixtures/qsjh2h_srp.txt").lines() {
        session.run_command(line);
    }
    let trainer = session.trainer.as_mut().expect("build_tree failed");
    trainer.train(&Accuracy::Fraction(0.1), TrainFinish::Indefinite);

    let (oop_ev, ip_ev) = solver_evs(trainer);
    assert!((oop_ev - 104.97).abs() < 0.5, "OOP EV {} != PIO 104.97", oop_ev);
    assert!((ip_ev - 75.03).abs() < 0.5, "IP EV {} != PIO 75.03", ip_ev);
}
