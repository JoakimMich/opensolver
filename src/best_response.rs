use crate::range::*;
use crate::postfloptree::*;
use crate::cfr::*;
use crate::hand_range::*;
use rust_poker::constants::RANK_TO_CHAR;
use rust_poker::constants::SUIT_TO_CHAR;
use rust_poker::hand_range::{get_card_mask};
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
        let mut total_ev = 0.0;
        
        let villain_pos = pos ^ true;
        let board = &self.range_manager.initial_board;
        let board_mask = get_card_mask(&board);
        let hero_hands = self.range_manager.get_num_hands(pos, board_mask, None);
        let hero_range = &self.range_manager.get_range(pos, board_mask, None).hands;
        let villain_range = &self.range_manager.get_range(villain_pos, board_mask, None).hands;
        
        let relative_probs = match pos {
            true => &self.oop_relative_probs,
            false => &self.ip_relative_probs,
        };
        let villain_reach_probs = self.range_manager.get_initial_reach_probs(villain_pos);
        
        let mut ev_results = vec![];
        let mut new_br = BestResponseState::new(self.range_manager, &mut ev_results, root, pos, &villain_reach_probs, (board_mask, None));
        new_br.run();
        
        for i in 0..hero_hands {
            total_ev += ev_results[i] / get_unblocked_count(hero_range[i], villain_range) * relative_probs[i];
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
    
    pub fn print_exploitability(&mut self, root: &Node) -> f64 {
        let oop_ev = self.get_best_response_ev(true, root);
        let ip_ev = self.get_best_response_ev(false, root);
        
        let exploitability = (oop_ev/2.0 + ip_ev/2.0) / 2.0 / (root.pot_size as f64) * 100.0;
        
        println!("OOP Best Response EV: {}", oop_ev/2.0 + (root.pot_size as f64 / 2.0) );
        println!("IP Best Response EV: {}", ip_ev/2.0 + (root.pot_size as f64 / 2.0));
        println!("Exploitability: {}% \n", exploitability);
        
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

struct BestResponseState<'a> {
    range_manager: &'a RangeManager,
    result: &'a mut Vec<f64>,
    node: &'a Node,
    oop: bool,
    villain_reach_probs: &'a Vec<f64>,
    board_masks: (u64, Option<u64>),
}

fn recursive_br(range_manager: &RangeManager, results: &mut Vec<f64>, child: &Node, oop: bool, villain_reach_probs: &Vec<f64>, board_masks: (u64, Option<u64>)) {
    let mut new_br = BestResponseState::new(range_manager, results, child, oop, villain_reach_probs, board_masks);
    new_br.run();
}

impl<'a> BestResponseState<'a> {
    fn new(range_manager: &'a RangeManager, result: &'a mut Vec<f64>, node: &'a Node, oop: bool, villain_reach_probs: &'a Vec<f64>, board_masks: (u64, Option<u64>) ) -> BestResponseState<'a> {
        BestResponseState { range_manager, result, node, oop, villain_reach_probs, board_masks }
    }
    
    pub fn run(&mut self) {       
        match self.node.node_type {
            NodeType::TerminalNode(terminal_type) => {
                *self.result = get_payoffs(self.oop, self.range_manager, self.board_masks, self.node, self.villain_reach_probs, &terminal_type);
            },
            NodeType::ChanceNode(deck_left) => { 
                let hero_hands = if self.oop == true {
                    self.node.oop_num_hands
                } else {
                    self.node.ip_num_hands
                };
                
                *self.result = vec![0.0; hero_hands];
                let results: Vec<_> = self.node.children.par_iter()
                                                        .map(|val| {
                                                            let new_masks = match val.node_type {
                                                                    NodeType::ChanceNodeCard((new,old)) => (new,old),
                                                                    _ => panic!("panicando!"),
                                                                };
                                                            
                                                            let mut results = vec![0.0; hero_hands];
                                                            if deck_left == 0 {
                                                                recursive_br(self.range_manager, &mut results, val, self.oop, self.villain_reach_probs, new_masks);
                                                            } else {
                                                                let new_villain_reach_prob = self.range_manager.get_villain_reach(self.oop, new_masks.0, new_masks.1, self.villain_reach_probs);
                                                                recursive_br(self.range_manager, &mut results, val, self.oop, &new_villain_reach_prob, new_masks);
                                                            }
                                                            results
                                                        })
                                                        .collect();
        
                if deck_left != 0 {
                    for (count,child) in self.node.children.iter().enumerate() {
                        let new_masks = match child.node_type {
                            NodeType::ChanceNodeCard((new,old)) => (new,old),
                            _ => panic!("panicando!"),
                        };
                        let reach_mapping = self.range_manager.get_reach_mapping(self.oop, new_masks.0, new_masks.1);
                        
                        for (i, mapping) in reach_mapping.iter().enumerate() {
                            self.result[*mapping as usize] += results[count][i] * (1.0/deck_left as f64);
                        }
                    }
                } else {
                    for i in 0..hero_hands {
                        for (count,_) in self.node.children.iter().enumerate() {
                            self.result[i] += results[count][i];
                        }
                    }
                }
            }, 
            NodeType::ChanceNodeCard(_) => { 
                let mut new_br = BestResponseState::new(self.range_manager, self.result,  &self.node.children[0], self.oop, self.villain_reach_probs, self.board_masks);
                new_br.run();
            }, 
            NodeType::ActionNode(ref node_info) => {
                let n_actions = node_info.actions_num;                
                if node_info.oop == self.oop {
                    let hero_hands = if self.oop == true {
                        self.node.oop_num_hands
                    } else {
                        self.node.ip_num_hands
                    };
                
                    *self.result = vec![f64::MIN; hero_hands];
   
                    let results: Vec<_> = self.node.children.par_iter()
                                                            .map(|val| {
                                                                let mut results = vec![0.0; hero_hands];
                                                                recursive_br(self.range_manager, &mut results, val, self.oop, self.villain_reach_probs, self.board_masks);
                                                                results
                                                            })
                                                            .collect();
                    
                    for (i,result) in self.result.iter_mut().enumerate() {
                        for results_j in results.iter() {
                            if results_j[i] > *result {
                                *result = results_j[i];
                            }
                        }
                    }
                    
                    
                } else {
                    let villain_pos = self.oop ^ true;
                    let average_strategy = node_info.get_average_strategy();
                    let hero_hands = if self.oop == true {
                        self.node.oop_num_hands
                    } else {
                        self.node.ip_num_hands
                    };
                    let villain_hands = if self.oop == true {
                        self.node.ip_num_hands
                    } else {
                        self.node.oop_num_hands
                    };               
                    *self.result = vec![0.0; hero_hands];
       
                    let results: Vec<_> = self.node.children.par_iter()
                                                            .enumerate()
                                                            .map(|(count, val)| {
                                                                let mut results = vec![0.0; hero_hands];
                                                                let mut offset = 0;
                                                                let mut new_villain_reach_prob = vec![0.0; villain_hands];
                                                                for (i, reach_prob) in new_villain_reach_prob.iter_mut().enumerate() {
                                                                    *reach_prob = average_strategy[offset+count] * self.villain_reach_probs[i];
                                                                    
                                                                    offset += n_actions;
                                                                }
                                                                recursive_br(self.range_manager, &mut results, val, self.oop, &new_villain_reach_prob, self.board_masks);
                                                                results
                                                            })
                                                            .collect();
                    
                    for (i, result) in self.result.iter_mut().enumerate() {
                        for results_j in results.iter() {
                            *result += results_j[i];
                        }
                    }
                                        
                }
                
            },
        }
    }
}