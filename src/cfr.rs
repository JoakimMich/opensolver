use crate::range::*;
use crate::postfloptree::*;
use crate::hand_range::Combo;
use rayon::prelude::*;

pub struct CfrState<'a> {
    range_manager: &'a RangeManager,
    result: &'a mut Vec<f64>,
    node: &'a mut Node,
    oop: bool,
    villain_reach_probs: &'a Vec<f64>,
    board_masks: (u64, Option<u64>),
    n_iterations: u64,
}

impl<'a> CfrState<'a> {
    pub fn new(range_manager: &'a RangeManager, result: &'a mut Vec<f64>, node: &'a mut Node, oop: bool, villain_reach_probs: &'a Vec<f64>, board_masks: (u64, Option<u64>), n_iterations: u64) -> CfrState<'a> {
        CfrState { range_manager, result, node, oop, villain_reach_probs, board_masks, n_iterations }
    }

    /// Runs one CFR pass for `oop` over the tree, leaving the hero's counterfactual values in `result`
    pub fn run(&mut self) {
        let ctx = Ctx { range_manager: self.range_manager, oop: self.oop, n_iterations: self.n_iterations };
        let (hero_range, villain_range) = ctx.ranges(self.board_masks);
        // CFR runs in f32
        let villain_reach_probs: Vec<f32> = self.villain_reach_probs.iter().map(|&x| x as f32).collect();
        let mut result = vec![0.0f32; hero_range.len()];
        cfr(&ctx, &mut result, self.node, &villain_reach_probs, hero_range, villain_range);
        *self.result = result.iter().map(|&x| x as f64).collect();
    }
}

/// f32 for CFR, f64 for best response
pub trait Float: Copy + Default + std::ops::Add<Output = Self> + std::ops::Sub<Output = Self> + std::ops::Mul<Output = Self> + std::ops::AddAssign + std::ops::SubAssign {}
impl Float for f32 {}
impl Float for f64 {}

/// Values that stay fixed during a CFR pass
struct Ctx<'a> {
    range_manager: &'a RangeManager,
    oop: bool,
    n_iterations: u64,
}

impl<'a> Ctx<'a> {
    /// (hero range, villain range) on a board
    #[inline]
    fn ranges(&self, board_masks: (u64, Option<u64>)) -> (&'a [Combo], &'a [Combo]) {
        let range_manager = self.range_manager;
        (
            &range_manager.get_range(self.oop, board_masks.0, board_masks.1).hands,
            &range_manager.get_range(!self.oop, board_masks.0, board_masks.1).hands,
        )
    }
}

/// Writes the hero's counterfactual value of every hand at `node` into `result` (len = hero hands).
/// `hero_range`/`villain_range` are the ranges on the current board.
fn cfr(ctx: &Ctx, result: &mut [f32], node: &mut Node, villain_reach_probs: &[f32], hero_range: &[Combo], villain_range: &[Combo]) {
    match node.node_type {
        NodeType::TerminalNode(TerminalType::TerminalShowdown) => {
            showdown_payoffs(result, hero_range, villain_range, villain_reach_probs, node.pot_size as f32);
        },
        NodeType::TerminalNode(TerminalType::TerminalFold(fold_position)) => {
            let value = if ctx.oop == fold_position { -(node.pot_size as f32) } else { node.pot_size as f32 };
            fold_payoffs(result, hero_range, villain_range, villain_reach_probs, value);
        },
        NodeType::ChanceNodeCard(_) => {
            cfr(ctx, result, &mut node.children[0], villain_reach_probs, hero_range, villain_range);
        },
        NodeType::ChanceNode(deck_left) => {
            // Cards are dealt in parallel; this is where all of the solver's parallelism comes from
            let oop = ctx.oop;
            let child_results: Vec<Vec<f32>> = node.children.par_iter_mut()
                .map(|child| {
                    let board_masks = match child.node_type {
                        NodeType::ChanceNodeCard(board_masks) => board_masks,
                        _ => unreachable!(),
                    };
                    let (hero_range, villain_range) = ctx.ranges(board_masks);
                    let mut results = vec![0.0; hero_range.len()];
                    if deck_left == 0 {
                        cfr(ctx, &mut results, child, villain_reach_probs, hero_range, villain_range);
                    } else {
                        let reach_mapping = ctx.range_manager.get_reach_mapping(!oop, board_masks.0, board_masks.1);
                        let new_villain_reach_probs: Vec<f32> = reach_mapping.iter().map(|&m| unsafe { *villain_reach_probs.get_unchecked(m as usize) }).collect();
                        cfr(ctx, &mut results, child, &new_villain_reach_probs, hero_range, villain_range);
                    }
                    results
                })
                .collect();

            result.fill(0.0);
            if deck_left != 0 {
                let scale = 1.0/deck_left as f32;
                for (child, results) in node.children.iter().zip(&child_results) {
                    let board_masks = match child.node_type {
                        NodeType::ChanceNodeCard(board_masks) => board_masks,
                        _ => unreachable!(),
                    };
                    let reach_mapping = ctx.range_manager.get_reach_mapping(oop, board_masks.0, board_masks.1);
                    for (&mapping, &value) in reach_mapping.iter().zip(results) {
                        unsafe { *result.get_unchecked_mut(mapping as usize) += value * scale; }
                    }
                }
            } else {
                for (i, value) in result.iter_mut().enumerate() {
                    for results in &child_results {
                        *value += results[i];
                    }
                }
            }
        },
        NodeType::ActionNode(ref mut node_info) => {
            let n_actions = node_info.actions_num;
            if n_actions == 1 {
                cfr(ctx, result, &mut node.children[0], villain_reach_probs, hero_range, villain_range);
                return;
            }

            let strategy = node_info.get_current_strategy();
            let hero_hands = result.len();

            if node_info.oop == ctx.oop {
                // Hero acts: value is the strategy-weighted value of the actions
                let mut action_results = vec![0.0; n_actions * hero_hands];
                for (child, results) in node.children.iter_mut().zip(action_results.chunks_exact_mut(hero_hands)) {
                    cfr(ctx, results, child, villain_reach_probs, hero_range, villain_range);
                }

                result.fill(0.0);
                for (probs, results) in strategy.chunks_exact(hero_hands).zip(action_results.chunks_exact(hero_hands)) {
                    for ((value, &prob), &action_value) in result.iter_mut().zip(probs).zip(results) {
                        *value += prob * action_value;
                    }
                }

                node_info.update_regret_sum(&action_results, result, ctx.n_iterations);
            } else {
                // Villain acts: sum over actions, with the villain's reach scaled by its strategy
                let mut new_villain_reach_probs = vec![0.0; villain_reach_probs.len()];
                let mut results = vec![0.0; hero_hands];
                result.fill(0.0);

                for (child, probs) in node.children.iter_mut().zip(strategy.chunks_exact(villain_reach_probs.len())) {
                    for ((reach_prob, &prob), &villain_reach_prob) in new_villain_reach_probs.iter_mut().zip(probs).zip(villain_reach_probs) {
                        *reach_prob = prob * villain_reach_prob;
                    }
                    cfr(ctx, &mut results, child, &new_villain_reach_probs, hero_range, villain_range);
                    for (value, child_value) in result.iter_mut().zip(&results) {
                        *value += child_value;
                    }
                }

                node_info.update_strategy_sum(&strategy, villain_reach_probs, ctx.n_iterations);
            }
        },
    }
}

/// Showdown: win the pot against weaker villain hands, lose it against stronger ones.
/// Both ranges are sorted by hand strength; card sums remove villain combos blocked by the hero.
#[inline]
pub fn showdown_payoffs<T: Float>(result: &mut [T], hero_range: &[Combo], villain_range: &[Combo], villain_reach_probs: &[T], value: T) {
    let villain_hands = villain_range.len();
    unsafe {
        let mut card_sum_win = [T::default(); 52];
        let mut sum_win = T::default();
        let mut j = 0;
        for (i, hero_combo) in hero_range.iter().enumerate() {
            while j < villain_hands && villain_range.get_unchecked(j).3 < hero_combo.3 {
                let villain_combo = villain_range.get_unchecked(j);
                let reach_prob = *villain_reach_probs.get_unchecked(j);
                sum_win += reach_prob;
                *card_sum_win.get_unchecked_mut(villain_combo.0 as usize) += reach_prob;
                *card_sum_win.get_unchecked_mut(villain_combo.1 as usize) += reach_prob;
                j += 1;
            }
            *result.get_unchecked_mut(i) = (sum_win - *card_sum_win.get_unchecked(hero_combo.0 as usize) - *card_sum_win.get_unchecked(hero_combo.1 as usize)) * value;
        }

        let mut card_sum_lose = [T::default(); 52];
        let mut sum_lose = T::default();
        let mut j = villain_hands;
        for i in (0..hero_range.len()).rev() {
            let hero_combo = hero_range.get_unchecked(i);
            while j > 0 && villain_range.get_unchecked(j-1).3 > hero_combo.3 {
                let villain_combo = villain_range.get_unchecked(j-1);
                let reach_prob = *villain_reach_probs.get_unchecked(j-1);
                sum_lose += reach_prob;
                *card_sum_lose.get_unchecked_mut(villain_combo.0 as usize) += reach_prob;
                *card_sum_lose.get_unchecked_mut(villain_combo.1 as usize) += reach_prob;
                j -= 1;
            }
            *result.get_unchecked_mut(i) -= (sum_lose - *card_sum_lose.get_unchecked(hero_combo.0 as usize) - *card_sum_lose.get_unchecked(hero_combo.1 as usize)) * value;
        }
    }
}

/// Fold: `value` against every villain combo that doesn't share a card with the hero combo
#[inline]
pub fn fold_payoffs<T: Float>(result: &mut [T], hero_range: &[Combo], villain_range: &[Combo], villain_reach_probs: &[T], value: T) {
    unsafe {
        let mut villain_card_sum = [T::default(); 52];
        let mut villain_sum = T::default();
        for (villain_combo, &reach_prob) in villain_range.iter().zip(villain_reach_probs) {
            *villain_card_sum.get_unchecked_mut(villain_combo.0 as usize) += reach_prob;
            *villain_card_sum.get_unchecked_mut(villain_combo.1 as usize) += reach_prob;
            villain_sum += reach_prob;
        }

        for (value_out, hero_combo) in result.iter_mut().zip(hero_range) {
            // the identical combo was subtracted twice above
            let villain_reach = match hero_combo.4 {
                Some(idx) => *villain_reach_probs.get_unchecked(idx as usize),
                None => T::default(),
            };
            *value_out = (villain_sum - *villain_card_sum.get_unchecked(hero_combo.0 as usize) - *villain_card_sum.get_unchecked(hero_combo.1 as usize) + villain_reach) * value;
        }
    }
}
