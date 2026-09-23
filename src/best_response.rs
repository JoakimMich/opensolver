use crate::range::*;
use crate::postfloptree::*;
use crate::cfr::{showdown_payoffs, fold_payoffs};
use crate::hand_range::*;
use crate::cards::get_card_mask;
use rayon::prelude::*;

pub struct BestResponse<'a> {
    range_manager: &'a RangeManager,
    pub oop_relative_probs: Vec<f64>,
    pub ip_relative_probs: Vec<f64>,
}

impl<'a> BestResponse<'a> {
    pub fn new(range_manager: &'a RangeManager) -> BestResponse<'a> {
        let board = &range_manager.initial_board;
        let board_mask = get_card_mask(&board);
        let oop_hands = range_manager.get_num_hands(true, board_mask, None);
        let ip_hands = range_manager.get_num_hands(false, board_mask, None);
        let oop_relative_probs = vec![0.0; oop_hands];
        let ip_relative_probs = vec![0.0; ip_hands];
        BestResponse { range_manager, oop_relative_probs, ip_relative_probs }
    }
    
    pub fn get_best_response_ev(&mut self, pos: bool, root: &Node) -> f64 {
        let board_mask = get_card_mask(&self.range_manager.initial_board);
        let hero_range = &self.range_manager.get_range(pos, board_mask, None).hands;
        let villain_range = &self.range_manager.get_range(!pos, board_mask, None).hands;

        let relative_probs = match pos {
            true => &self.oop_relative_probs,
            false => &self.ip_relative_probs,
        };
        let villain_reach_probs: Vec<f32> = self.range_manager.get_initial_reach_probs(!pos).iter().map(|&x| x as f32).collect();

        let ctx = Ctx { range_manager: self.range_manager, oop: pos };
        let mut ev_results = vec![0.0f32; hero_range.len()];
        best_response(&ctx, &mut ev_results, root, &villain_reach_probs, hero_range, villain_range);

        let mut total_ev = 0.0;
        for (i, &ev) in ev_results.iter().enumerate() {
            total_ev += ev as f64 / get_unblocked_count(hero_range[i], villain_range) * relative_probs[i];
        }

        total_ev
    }

    pub fn set_relative_probablities(&mut self, pos: bool) {
        let villain_pos = pos ^ true;
        let board = &self.range_manager.initial_board;
        let board_mask = get_card_mask(&board);
        let hero_hands = self.range_manager.get_num_hands(pos, board_mask, None);
        let hero_range = &self.range_manager.get_range(pos, board_mask, None).hands;
        let villain_range = &self.range_manager.get_range(villain_pos, board_mask, None).hands;
        
        let relative_probs = match pos {
            true => &mut self.oop_relative_probs,
            false => &mut self.ip_relative_probs,
        };
        let mut relative_sum = 0.0;
        
        for i in 0..hero_hands {
            let hero_combo = hero_range[i];
            let mut villain_sum = 0.0;
            
            for villain_combo in villain_range.iter() {
                if overlap_combos(hero_combo, *villain_combo) {
                    continue;
                }
                
                villain_sum += villain_combo.2 as f64 / 100.0;
            }
            
            relative_probs[i] = villain_sum * (hero_combo.2 as f64 / 100.0);
            relative_sum += relative_probs[i];
        }
        for i in relative_probs {
            *i /= relative_sum;
        }
        
    }
    
    pub fn print_exploitability(&mut self, root: &Node, time_elapsed: f64) -> f64 {
        let oop_ev = self.get_best_response_ev(true, root);
        let ip_ev = self.get_best_response_ev(false, root);
        
        let exploitability = (oop_ev/2.0 + ip_ev/2.0) / 2.0;
        println!("SOLVER:");
        println!("running time: {}", time_elapsed);
        println!("OOP's MES: {}", oop_ev/2.0 + (root.pot_size as f64 / 2.0) );
        println!("IP's MES: {}", ip_ev/2.0 + (root.pot_size as f64 / 2.0));
        println!("Exploitable for: {} ({}%)", exploitability, exploitability / (root.pot_size as f64) * 100.0);
        println!("END \n");
        exploitability
    }
}

fn get_unblocked_count(hero_combo: Combo, villain_range: &Vec<Combo>) -> f64 {
    let mut sum = 0.0;
    for villain_combo in villain_range {
        if !overlap_combos(*villain_combo, hero_combo) {
            sum += villain_combo.2 as f64 / 100.0;
        }
    }
    sum
}

fn overlap_combos(hero_combo: Combo, villain_combo: Combo) -> bool {
    if hero_combo.0 == villain_combo.0 || hero_combo.0 == villain_combo.1 {
        return true;
    }
    if hero_combo.1 == villain_combo.0 || hero_combo.1 == villain_combo.1 {
        return true;
    }
    
    false
}

/// Values that stay fixed during a best response pass
struct Ctx<'a> {
    range_manager: &'a RangeManager,
    oop: bool,
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

/// Writes the value of every hero hand at `node` into `result` when the hero plays a best response
/// against the villain's average strategy
fn best_response(ctx: &Ctx, result: &mut [f32], node: &Node, villain_reach_probs: &[f32], hero_range: &[Combo], villain_range: &[Combo]) {
    match node.node_type {
        NodeType::TerminalNode(TerminalType::TerminalShowdown) => {
            showdown_payoffs(result, hero_range, villain_range, villain_reach_probs, node.pot_size as f32);
        },
        NodeType::TerminalNode(TerminalType::TerminalFold(fold_position)) => {
            let value = if ctx.oop == fold_position { -(node.pot_size as f32) } else { node.pot_size as f32 };
            fold_payoffs(result, hero_range, villain_range, villain_reach_probs, value);
        },
        NodeType::ChanceNodeCard(_) => {
            best_response(ctx, result, &node.children[0], villain_reach_probs, hero_range, villain_range);
        },
        NodeType::ChanceNode(deck_left) => {
            let oop = ctx.oop;
            let child_results: Vec<Vec<f32>> = node.children.par_iter()
                .map(|child| {
                    let board_masks = match child.node_type {
                        NodeType::ChanceNodeCard(board_masks) => board_masks,
                        _ => unreachable!(),
                    };
                    let (hero_range, villain_range) = ctx.ranges(board_masks);
                    let mut results = vec![0.0f32; hero_range.len()];
                    if deck_left == 0 {
                        best_response(ctx, &mut results, child, villain_reach_probs, hero_range, villain_range);
                    } else {
                        let reach_mapping = ctx.range_manager.get_reach_mapping(!oop, board_masks.0, board_masks.1);
                        let new_villain_reach_probs: Vec<f32> = reach_mapping.iter().map(|&m| unsafe { *villain_reach_probs.get_unchecked(m as usize) }).collect();
                        best_response(ctx, &mut results, child, &new_villain_reach_probs, hero_range, villain_range);
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
        NodeType::ActionNode(ref node_info) => {
            if node_info.actions_num == 1 {
                // a single action is always taken, whoever acts
                best_response(ctx, result, &node.children[0], villain_reach_probs, hero_range, villain_range);
                return;
            }

            let mut results = vec![0.0f32; result.len()];
            if node_info.oop == ctx.oop {
                // Hero picks the best action for every hand
                best_response(ctx, result, &node.children[0], villain_reach_probs, hero_range, villain_range);
                for child in &node.children[1..] {
                    best_response(ctx, &mut results, child, villain_reach_probs, hero_range, villain_range);
                    for (value, &child_value) in result.iter_mut().zip(&results) {
                        *value = value.max(child_value);
                    }
                }
            } else {
                // Villain plays its average strategy
                let average_strategy = node_info.get_average_strategy_by_action();
                let mut new_villain_reach_probs = vec![0.0f32; villain_reach_probs.len()];
                result.fill(0.0);
                for (child, probs) in node.children.iter().zip(average_strategy.chunks_exact(villain_reach_probs.len())) {
                    for ((reach_prob, &prob), &villain_reach_prob) in new_villain_reach_probs.iter_mut().zip(probs).zip(villain_reach_probs) {
                        *reach_prob = prob * villain_reach_prob;
                    }
                    best_response(ctx, &mut results, child, &new_villain_reach_probs, hero_range, villain_range);
                    for (value, &child_value) in result.iter_mut().zip(&results) {
                        *value += child_value;
                    }
                }
            }
        },
    }
}
