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

// Isomorphism: skipping isomorphic cards must not change the game. Solving with and without it
// runs the same algorithm, so after the same iterations EVs and strategies agree up to rounding.
mod isomorphism {
    use super::*;
    use crate::cards::get_card_mask;
    use crate::postfloptree::NodeInfo;
    use crate::upi::CliSession;

    const LINES: [&[u32]; 7] = [&[0, 0, 0, 0, 0, 0], &[50, 50, 0, 0, 0, 0], &[0, 50, 50, 0, 0, 0, 0], &[0, 0, 60, 60, 0, 0], &[0, 0, 0, 60, 60, 0, 0], &[0, 0, 0, 0, 80, 80], &[0, 0, 0, 0, 0, 80, 80]];

    fn solve(board: &str, isomorphism: bool, iterations: u64) -> Trainer {
        let mut range_manager = RangeManager::new(HandRange::from_string("AA,KK,QQ,AK,T9s,87s,A5s".to_string()), HandRange::from_string("KK,JJ,AQ,KQ,98s,76s,A4s".to_string()), board.to_string());
        range_manager.isomorphism = isomorphism;
        let mut trainer = Trainer::new(range_manager, LINES.iter().map(|l| l.to_vec()).collect(), 300, 100);
        trainer.train(&Accuracy::Chips(0.0), TrainFinish::Iterations(iterations));
        trainer
    }

    fn assert_close(a: &[f64], b: &[f64], what: &str) {
        assert_eq!(a.len(), b.len(), "{}", what);
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() < 1e-4, "{}: {} vs {}", what, x, y);
        }
    }

    fn children_summary(children: Vec<NodeInfo>) -> Vec<(String, String, (u32, u32, u32), u32)> {
        children.into_iter().map(|c| (c.line, c.board, c.pot, c.children_count)).collect()
    }

    // Regret matching jumps from uniform to pure play when a regret crosses zero, so rounding
    // differences between the two solves eventually grow. Strategies are compared after a few
    // iterations (where they agree to rounding, 1e-15 in f64), EVs again after converging.
    fn check(board: &str, lines: &[&str], dealt_turns: usize) {
        let full = solve(board, false, 4);
        let iso = solve(board, true, 4);
        assert_eq!(iso.range_manager.get_board_deck(get_card_mask(board)).len(), dealt_turns);

        let (full_oop, full_ip) = solver_evs(&full);
        let (iso_oop, iso_ip) = solver_evs(&iso);
        assert!((full_oop - iso_oop).abs() < 1e-3 && (full_ip - iso_ip).abs() < 1e-3, "EVs {} {} vs {} {}", full_oop, full_ip, iso_oop, iso_ip);

        let map = CliSession::new().hand_order_map;
        for line in lines {
            let line = line.to_string();
            for (a, b) in full.root.get_strategy(line.clone(), &full.range_manager, &map).iter().zip(iso.root.get_strategy(line.clone(), &iso.range_manager, &map)) {
                assert_close(a, &b, &format!("strategy {}", line));
            }
            for oop in [true, false] {
                assert_close(&full.root.get_range(oop, line.clone(), &full.range_manager, &map), &iso.root.get_range(oop, line.clone(), &iso.range_manager, &map), &format!("range {}", line));
            }
            let parent = line.rsplit_once(':').unwrap().0.to_string();
            assert_eq!(children_summary(full.root.get_children(parent.clone(), &full.range_manager)), children_summary(iso.root.get_children(parent, &iso.range_manager)));
        }

        let (full_oop, full_ip) = solver_evs(&solve(board, false, 150));
        let (iso_oop, iso_ip) = solver_evs(&solve(board, true, 150));
        assert!((full_oop - iso_oop).abs() < 0.05 && (full_ip - iso_ip).abs() < 0.05, "converged EVs {} {} vs {} {}", full_oop, full_ip, iso_oop, iso_ip);
    }

    #[test]
    fn two_tone_flop() {
        // clubs are skipped and play like diamonds
        check("QsJh2h", &["r:0:c:c:Kc", "r:0:c:c:Kc:c:c:Ac", "r:0:c:c:Kd:c:c:3c", "r:0:b50:c:9c:c:c:9d"], 36);
    }

    #[test]
    fn monotone_flop() {
        // diamonds and clubs are skipped and play like spades
        check("QhJh2h", &["r:0:c:c:Kc", "r:0:c:c:Kd:c:c:Ac", "r:0:c:c:Kc:c:c:As", "r:0:c:b50:c:5d:c:c:5c"], 23);
    }

    #[test]
    fn paired_flop() {
        // spades and diamonds hold the same ranks (a 7 each)
        check("7s7dKh", &["r:0:c:c:Ad", "r:0:c:c:Ad:c:c:Ac", "r:0:c:c:2c:c:c:2d"], 37);
    }
}
