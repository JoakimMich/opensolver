use crate::range::*;
use crate::postfloptree::*;
use rust_poker::constants::RANK_TO_CHAR;
use rust_poker::constants::SUIT_TO_CHAR;
use rayon::prelude::*;

pub struct CfrState<'a> {
    range_manager: &'a RangeManager<'a>,
    result: &'a mut Vec<f64>,
    node: &'a mut Node,
    oop: bool,
    villain_reach_probs: &'a Vec<f64>,
    board: &'a str,
    n_iterations: u64,
}

fn recursive_cfr(range_manager: &RangeManager, results: &mut Vec<f64>, child: &mut Node, oop: bool, villain_reach_probs: &Vec<f64>, board: &str, n_iterations: u64) {
    let mut new_cfr = CfrState::new(range_manager, results, child, oop, villain_reach_probs, board, n_iterations);
    new_cfr.run();
}

impl<'a> CfrState<'a> {
    pub fn new(range_manager: &'a RangeManager, result: &'a mut Vec<f64>, node: &'a mut Node, oop: bool, villain_reach_probs: &'a Vec<f64>, board: &'a str, n_iterations: u64) -> CfrState<'a> {
        CfrState { range_manager, result, node, oop, villain_reach_probs, board, n_iterations }
    }
    pub fn run(&mut self) {       
        match self.node.node_type {
            NodeType::TerminalNode(terminal_type) => {
                *self.result = get_payoffs(self.oop, self.range_manager, self.board, self.node, self.villain_reach_probs, &terminal_type);
            },
            NodeType::ChanceNode(deck_left) => {
                let hero_hands = self.range_manager.get_num_hands(self.oop, self.board);
                *self.result = vec![0.0; hero_hands];
                
                let results: Vec<_> = self.node.children.par_iter_mut()
                                                            .map(|val| {
                                                                let next_card = match val.node_type {
                                                                    NodeType::ChanceNodeCard(card) => card,
                                                                    _ => panic!("panicando!"),
                                                                };
                                                                
                                                                let next_board = match next_card {
                                                                    Some(card) => {
                                                                        let rank = RANK_TO_CHAR[usize::from(card >> 2)];
                                                                        let suit = SUIT_TO_CHAR[usize::from(card & 3)];
                                                                        let mut new_board = self.board.to_string();
                                                                        new_board.push(rank);
                                                                        new_board.push(suit);
                                                                        new_board
                                                                    },
                                                                    _ => self.board.to_string(),
                                                                };
                                                                let next_board = next_board.as_str();
                                                                
                                                                let mut results = vec![0.0; hero_hands];
                                                                if deck_left == 0 {
                                                                    recursive_cfr(self.range_manager, &mut results, val, self.oop, self.villain_reach_probs, next_board, self.n_iterations);
                                                                } else {
                                                                    let new_villain_reach_prob = self.range_manager.get_villain_reach(self.oop, next_board, self.villain_reach_probs);
                                                                    recursive_cfr(self.range_manager, &mut results, val, self.oop, &new_villain_reach_prob, next_board, self.n_iterations);
                                                                }
                                                                results
                                                            })
                                                            .collect();
                
                if deck_left != 0 {
                    for (count,child) in self.node.children.iter_mut().enumerate() {
                        let next_card_u8 = match child.node_type {
                            NodeType::ChanceNodeCard(card) => card,
                            _ => panic!("panicando"),
                        }.unwrap();
                        let rank = RANK_TO_CHAR[usize::from(next_card_u8 >> 2)];
                        let suit = SUIT_TO_CHAR[usize::from(next_card_u8 & 3)];
                        let mut new_board = self.board.to_string();
                        new_board.push(rank);
                        new_board.push(suit);
                        let new_board = new_board.as_str();
                        let reach_mapping = self.range_manager.get_reach_mapping(self.oop, new_board);
                        
                        for (i, mapping) in reach_mapping.iter().enumerate() {
                            self.result[*mapping as usize] += results[count][i] * (1.0/deck_left as f64);
                        }
                    }
                } else {
                    for i in 0..hero_hands {
                        for (count,_) in self.node.children.iter_mut().enumerate() {
                            self.result[i] += results[count][i];
                        }
                    }
                }
            },
            NodeType::ChanceNodeCard(_) => { 
                let mut new_cfr = CfrState::new(self.range_manager, self.result, &mut self.node.children[0], self.oop, self.villain_reach_probs, self.board, self.n_iterations);
                new_cfr.run();
            },            
            NodeType::ActionNode(ref mut node_info) => {
                let n_actions = node_info.actions_num;
                
                if node_info.oop == self.oop {
                    let hero_hands = self.range_manager.get_num_hands(self.oop, self.board);
					if n_actions == 1 {
						recursive_cfr(self.range_manager, self.result, &mut self.node.children[0], self.oop, self.villain_reach_probs, self.board, self.n_iterations);
					} else {
						let current_strategy = node_info.get_current_strategy();
						*self.result = vec![0.0; hero_hands];
		   
						let results: Vec<_> = self.node.children.par_iter_mut()
																.map(|val| {
																	let mut results = vec![0.0; hero_hands];
																	recursive_cfr(self.range_manager, &mut results, val, self.oop, self.villain_reach_probs, self.board, self.n_iterations);
																	results
																})
																.collect();
						
						
						for (i, results_i) in results.iter().enumerate() {
							node_info.update_regret_sum_1(results_i, i)
						}
						
						let mut offset = 0;
						
						for (i,result) in self.result.iter_mut().enumerate() {
							for j in 0..n_actions {
								*result += current_strategy[offset+j] * results[j][i];
							}
							offset += n_actions;
						}
						
						node_info.update_regret_sum_2(self.result, self.n_iterations);
				
					}
                    
                } else {
                    let villain_pos = self.oop ^ true;
                    let hero_hands = self.range_manager.get_num_hands(self.oop, self.board);
                    let villain_hands = self.range_manager.get_num_hands(villain_pos, self.board);        
					if n_actions == 1 {
						recursive_cfr(self.range_manager, self.result, &mut self.node.children[0], self.oop, self.villain_reach_probs, self.board, self.n_iterations);
					} else {
						*self.result = vec![0.0; hero_hands];
						let current_strategy = node_info.get_current_strategy();
						
						let results: Vec<_> = self.node.children.par_iter_mut()
																.enumerate()
																.map(|(count, val)| {
																	let mut results = vec![0.0; hero_hands];
																	let mut offset = 0;
																	let mut new_villain_reach_prob = vec![0.0; villain_hands];
																	for (i, reach_prob) in new_villain_reach_prob.iter_mut().enumerate() {
																		*reach_prob = current_strategy[offset+count] * self.villain_reach_probs[i];
																		
																		offset += n_actions;
																	}
																	recursive_cfr(self.range_manager, &mut results, val, self.oop, &new_villain_reach_prob, self.board, self.n_iterations);
																	results
																})
																.collect();
						
						for (i, result) in self.result.iter_mut().enumerate() {
							for results_j in results.iter() {
								*result += results_j[i];
							}
						}
						
						node_info.update_strategy_sum(&current_strategy, self.villain_reach_probs, self.n_iterations);
						
					}
                    
                }
                
            },
        }
    }
}

pub fn get_payoffs(oop: bool, range_manager: &RangeManager, board: &str, node: &Node, villain_reach_probs: &[f64], terminal_type: &TerminalType) -> Vec<f64> {
    let villain_pos = oop ^ true;
    let hero_hands = range_manager.get_num_hands(oop, board);
    let villain_hands = range_manager.get_num_hands(villain_pos, board);
    let hero_range = &range_manager.get_range(oop, board).hands;
    let villain_range = &range_manager.get_range(villain_pos, board).hands;
    
    let mut results_new = vec![0.0; hero_hands];
    
    
    match terminal_type {
        TerminalType::TerminalShowdown => {
            let value = node.pot_size as f64;
            let mut card_sum_win = vec![0.0; 52];
            let mut sum_win = 0.0;
            let mut j = 0;
            
            for (i,hero_combo) in hero_range.iter().enumerate() {
                while j < villain_hands && villain_range[j].3 < hero_combo.3  {
                    let villain_combo = villain_range[j];
                    sum_win += villain_reach_probs[j];
                    card_sum_win[villain_combo.0 as usize] += villain_reach_probs[j];
                    card_sum_win[villain_combo.1 as usize] += villain_reach_probs[j];
                    j += 1;
                }
                results_new[i] = (sum_win - card_sum_win[hero_combo.0 as usize] - card_sum_win[hero_combo.1 as usize]) * value;
            }
            let mut card_sum_lose = vec![0.0; 52];
            let mut sum_lose = 0.0;
            let mut j = villain_hands;
            
            for i in (0..hero_hands).rev() {
                let hero_combo = hero_range[i];
                
                while j > 0 && villain_range[j-1].3 > hero_combo.3 {
                    let villain_combo = villain_range[j-1];
                    sum_lose += villain_reach_probs[j-1];
                    card_sum_lose[villain_combo.0 as usize] += villain_reach_probs[j-1];
                    card_sum_lose[villain_combo.1 as usize] += villain_reach_probs[j-1];
                    j -= 1;
                }
                results_new[i] -= (sum_lose - card_sum_lose[hero_combo.0 as usize] - card_sum_lose[hero_combo.1 as usize]) * value;
            }
        },
        TerminalType::TerminalFold(fold_position) => {
            let mut villain_sum = 0.0;
            let mut villain_card_sum = vec![0.0; 52];
            
            
            let value = if oop == *fold_position {
                -(node.pot_size as f64)
            } else {
                node.pot_size as f64
            };
            
            for (i,villain_combo) in villain_range.iter().enumerate() {
                villain_card_sum[villain_combo.0 as usize] += villain_reach_probs[i];
                villain_card_sum[villain_combo.1 as usize] += villain_reach_probs[i];
                villain_sum += villain_reach_probs[i];
            }
            
            
            for (i,hero_combo) in hero_range.iter().enumerate() {
                let villain_reach = match hero_combo.4 {
                    Some(idx) => villain_reach_probs[idx as usize],
                    None => 0.0
                };
                results_new[i] = (villain_sum - villain_card_sum[hero_range[i].0 as usize] - villain_card_sum[hero_range[i].1 as usize] + villain_reach)*value;
            }
        },
    }
    
    results_new
}