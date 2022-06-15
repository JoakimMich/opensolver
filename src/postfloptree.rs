use crate::range::*;
use std::cmp::min;
use rust_poker::constants::*;
use rust_poker::hand_range::{get_card_mask};
use std::collections::HashMap;

// discount CFR params
const ALPHA: f64 = 1.5;
const BETA: f64 = 0.5;
const GAMMA: f64 = 2.0;


#[derive(Debug,Clone,Copy)]
pub enum TerminalType {
    TerminalShowdown,
    TerminalFold(bool),
}

#[derive(Debug)]
pub struct ActionNodeInfo {
    pub oop: bool,
    pub actions: Vec<ActionType>,
    pub strategy_sum: Vec<f64>,
    regret_sum: Vec<f64>,
    pub actions_num: usize,
    hands_num: usize,
}

impl ActionNodeInfo {
    pub fn new(oop: bool, actions: Vec<ActionType>, hands_num: usize) -> ActionNodeInfo {
        let actions_num = actions.len();
        let strategy_sum = vec![0.0; hands_num * actions_num];
        let regret_sum = strategy_sum.clone();
        
        ActionNodeInfo { oop, actions, strategy_sum, regret_sum, actions_num, hands_num }
    }
    
    pub fn get_current_strategy(&self) -> Vec<f64> {
        let mut strategy = self.regret_sum.clone();
        strategy.iter_mut().for_each(|x| *x = x.max(0.0));
     
        strategy.chunks_mut(self.actions_num).for_each(|slice| {
            let hand_sum_regrets: f64 = slice.iter().sum();
            if hand_sum_regrets > 0.0 {
                slice.iter_mut().for_each(|x| *x /= hand_sum_regrets);
            } else {
                slice.iter_mut().for_each(|x| *x = 1.0/self.actions_num as f64);
            }
        });
        
        strategy
    }
    
    pub fn update_regret_sum_1(&mut self, action_utilities: &[f64], n_action: usize) {
        let mut offset = 0;
        for utility in action_utilities.iter() {
            self.regret_sum[offset+n_action] += utility;
            offset += self.actions_num;
        }
    }
    
    pub fn update_regret_sum_2(&mut self, action_utilities: &[f64], n_iterations: u64) {
        let mut x = f64::powf(n_iterations as f64, ALPHA);
        x = x / (x + 1.0);
        let mut offset = 0;
        
        for utility in action_utilities.iter() {
            for j in 0..self.actions_num {
                self.regret_sum[offset+j] -= utility;
                if self.regret_sum[offset+j] > 0.0 {
                    self.regret_sum[offset+j] *= x;
                } else {
                    self.regret_sum[offset+j] *= BETA;
                }
            }
            offset += self.actions_num;
        }
    }
    
    pub fn update_strategy_sum(&mut self, strategy: &[f64], reach_probs: &[f64], n_iterations: u64 ) {
        let x = f64::powf(n_iterations as f64 / (n_iterations as f64 + 1.0), GAMMA);
        let mut offset = 0;
        for reach_prob in reach_probs.iter() {
            for j in 0..self.actions_num {
                self.strategy_sum[offset+j] += strategy[offset+j] * reach_prob;
                self.strategy_sum[offset+j] *= x;
            }
            
            offset += self.actions_num;
        }
    }
    
    pub fn get_average_strategy(&self) -> Vec<f64> {
        let mut average_strategy = vec![0.0; self.hands_num * self.actions_num];
        let mut offset = 0;
        
        for _ in 0..self.hands_num {
            let mut total = 0.0;
            
            for j in 0..self.actions_num {
                total += self.strategy_sum[offset+j];
            }
            
            if total > 0.0 {
                for j in 0..self.actions_num {
                    average_strategy[offset+j] = self.strategy_sum[offset+j] / total;
                }
            } else {
                for j in 0..self.actions_num {
                    average_strategy[offset+j] = 1.0/self.actions_num as f64;
                }
            }
            
            offset += self.actions_num;
        }
        
        average_strategy
    }

}

#[derive(Debug)]
pub enum NodeType {
    ActionNode(ActionNodeInfo),
    TerminalNode(TerminalType),
    ChanceNode(u8),
    ChanceNodeCard((u64, Option<u64>)),
}

#[derive(Debug)]
pub struct Node {
    pub node_type: NodeType,
    pub children: Vec<Node>,
    pub pot_size: u32,
    pub chance_start_stack: u32,
    pub oop_invested: u32,
    pub ip_invested: u32,
    pub chance_start_pot: u32,
    pub oop_num_hands: usize,
    pub ip_num_hands: usize,
}

impl Node {
    pub fn new_root(chance_start_stack: u32, pot_size: u32, oop_num_hands: usize, ip_num_hands: usize) -> Node {
        Node { node_type: NodeType::ChanceNode(0), children: vec![] , pot_size, chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: pot_size, oop_num_hands, ip_num_hands}
    }
}

#[derive(Debug,Clone,Copy,Eq, PartialEq)]
pub enum ActionType {
    Fold,
    Check,
    Call,
    Bet(u32),
    Raise{sizing: u32},
}

pub fn get_sizings(lines: Vec<Vec<u32>>) -> HashMap<String, Vec<ActionType>> {
    let mut sizing_mapping = HashMap::new();
    
    for line in lines.iter() {
        let mut latest_action = 0;
        let mut node_line = "r".to_string();
        let mut previous_invested = 0;
        
        for (i,action) in line.iter().enumerate() {
            if i == 0 && action != &0 {
                sizing_mapping
                    .entry(node_line.clone())
                    .or_insert(Vec::new())
                    .push(ActionType::Bet(*action));
                node_line.push_str(format!(":b{}",*action).as_str());
            } else if i != 0 && action != &0 {
                if latest_action != *action {
                    let new_sizing = *action - previous_invested;
                    let v: &str = node_line.as_str().split(':').collect::<Vec<&str>>().last().unwrap();
                    
                    if v.contains("b") == false && v.contains("R") == false {
                        sizing_mapping
                            .entry(node_line.clone())
                            .or_insert(Vec::new())
                            .push(ActionType::Bet(new_sizing));
                        node_line.push_str(format!(":b{}",new_sizing).as_str());
                    } else {
                        sizing_mapping
                            .entry(node_line.clone())
                            .or_insert(Vec::new())
                            .push(ActionType::Raise{sizing: new_sizing});
                        node_line.push_str(format!(":R{}",new_sizing).as_str());
                    }
                } else {
                    let v: &str = node_line.as_str().split(':').collect::<Vec<&str>>().last().unwrap();
                    if v.contains("x") || v.contains("c") {
                        node_line.push_str(":x");
                    } else {
                        node_line.push_str(":c");
                        previous_invested = *action;
                    }
                    
                }
            } else if latest_action == 0 {
                node_line.push_str(":x");
            }
            
            latest_action = *action;
        }
    }
    
    for (key,value) in sizing_mapping.iter_mut() {
        value.dedup();
    }
    
    sizing_mapping
}

pub fn recursive_build(latest_action: Option<ActionType>, sizing_mapping: &HashMap<String, Vec<ActionType>>, action_line: &String, current_node: &mut Node, range_manager: &RangeManager, current_board: &String) {
    match &current_node.node_type {
        NodeType::ChanceNode(_) => {
            match latest_action  {
                Some(_) => {
                    if current_board.len() == 6 {
                        // Flop, add new turn cards
                        let board_mask = get_card_mask(current_board);
                        for card in range_manager.get_board_deck(board_mask).iter() {
                            let rank = RANK_TO_CHAR[usize::from(card >> 2)];
                            let suit = SUIT_TO_CHAR[usize::from(card & 3)];
                            let mut new_board = current_board.clone();
                            new_board.push(rank);
                            new_board.push(suit);
                            let new_board_mask = get_card_mask(&new_board);
                            
                            let new_eff_stack = current_node.chance_start_stack - (current_node.pot_size - current_node.chance_start_pot)/2;
                            let mut node_new = Node { node_type: NodeType::ChanceNodeCard((new_board_mask, None)), children: vec![], pot_size: current_node.pot_size, chance_start_stack: new_eff_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.pot_size, oop_num_hands: range_manager.get_num_hands(true, board_mask, None), ip_num_hands: range_manager.get_num_hands(false, board_mask, None) };
                            recursive_build(None, sizing_mapping, action_line, &mut node_new, range_manager, &new_board);
                            current_node.children.push(node_new);
                        }
                        
                    } else if current_board.len() == 8 {
                        let board_mask = get_card_mask(current_board);
                        for card in range_manager.get_board_deck(board_mask).iter() {
                            let rank = RANK_TO_CHAR[usize::from(card >> 2)];
                            let suit = SUIT_TO_CHAR[usize::from(card & 3)];
                            let mut new_board = current_board.clone();
                            new_board.push(rank);
                            new_board.push(suit);
                            let new_board_mask = get_card_mask(&new_board);
                            
                            let old_board_mask = if range_manager.initial_board.len() == 6 {
                                Some(board_mask)
                            } else {
                                None
                            };
                            
                            let new_eff_stack = current_node.chance_start_stack - (current_node.pot_size - current_node.chance_start_pot)/2;
                            let mut node_new = Node { node_type: NodeType::ChanceNodeCard((new_board_mask, old_board_mask)), children: vec![], pot_size: current_node.pot_size, chance_start_stack: new_eff_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.pot_size, oop_num_hands: range_manager.get_num_hands(true, new_board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, new_board_mask, old_board_mask) };
                            recursive_build(None, sizing_mapping, action_line, &mut node_new, range_manager, &new_board);
                            current_node.children.push(node_new);
                        }
                    } else {
                        panic!("Current board must be either length of flop or turn");
                    }
                },
                None => {
                        let board_mask = get_card_mask(current_board);
                        let mut node_new = Node { node_type: NodeType::ChanceNodeCard((board_mask, None)), children: vec![], pot_size: current_node.pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, None), ip_num_hands: range_manager.get_num_hands(false, board_mask, None) };
                        let current_line = "r".to_string();
                        recursive_build(None, sizing_mapping, &current_line, &mut node_new, range_manager, current_board);
                        current_node.children.push(node_new);
                }
            };
        },
        NodeType::ChanceNodeCard(_) => {
            let mut actions_new = vec![ActionType::Check];
            match sizing_mapping.get(&action_line.to_owned()) {
                Some(action_sizings) => {
                    for sizing in action_sizings {
                        actions_new.push(*sizing);
                    }
                },
                None => (),
            };
            
            let board_mask = get_card_mask(current_board);
            let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                let turn_board = &current_board[0..8];
                Some(get_card_mask(&turn_board))
            } else {
                None
            };
            
            actions_new.dedup();
            let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(true, actions_new, range_manager.get_num_hands(true, board_mask, old_board_mask))), children: vec![], pot_size: current_node.pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask) };
            
            recursive_build(None, sizing_mapping, action_line, &mut node_new, range_manager, current_board);
            current_node.children.push(node_new);
        },
        NodeType::TerminalNode(_) => {
        },
        NodeType::ActionNode(node_info) => {
            let oop = node_info.oop;
            let actions = &node_info.actions;
            match latest_action {
                Some(ActionType::Bet(sizing)) => {
                    for action in actions {
                        match action {
                            ActionType::Fold => {
                                // Add terminal fold
                                let eff_pot_size = current_node.pot_size - (sizing - min(current_node.oop_invested,current_node.ip_invested));
                                let board_mask = get_card_mask(current_board);
                                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                                    let turn_board = &current_board[0..8];
                                    Some(get_card_mask(&turn_board))
                                } else {
                                    None
                                };

                                let mut node_new = Node { node_type: NodeType::TerminalNode(TerminalType::TerminalFold(oop)), children: vec![], pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask) };
                                recursive_build(Some(*action), sizing_mapping, action_line, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Call => {
                                // Add terminal call, or next street if turn/river
                                let eff_pot_size = current_node.pot_size + (sizing - min(current_node.oop_invested,current_node.ip_invested));
                                let node_type_new = if current_board.len() == 10 {
                                    NodeType::TerminalNode(TerminalType::TerminalShowdown)
                                } else {
                                    NodeType::ChanceNode((range_manager.get_board_deck(get_card_mask(current_board)).len() - 4).try_into().unwrap())
                                };
                                
                                let board_mask = get_card_mask(current_board);
                                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                                    let turn_board = &current_board[0..8];
                                    Some(get_card_mask(&turn_board))
                                } else {
                                    None
                                };
                                let action_line = format!("{}:c",action_line);
                                let mut node_new = Node { node_type: node_type_new, children: vec![], pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask) };
                                
                                recursive_build(Some(*action), sizing_mapping, &action_line, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Raise{sizing: action_sizing} => {
                                // Add actions accordingly for opponent, and new action node for him
                                let call_amount = sizing - min(current_node.oop_invested,current_node.ip_invested);
                                let pot_size_new = current_node.pot_size + (action_sizing - sizing) + call_amount;
                                let eff_stack_new = current_node.chance_start_stack - (action_sizing - sizing);
                                let mut oop_new = oop;
                                oop_new ^= true;
                                let mut actions_new = vec![ActionType::Fold, ActionType::Call];
                                let action_line = format!("{}:R{}",action_line,action_sizing);
                                
                                if eff_stack_new != 0 {
                                    
                                    match sizing_mapping.get(&action_line.to_owned()) {
                                        Some(action_sizings) => {
                                            for sizing in action_sizings {
                                                actions_new.push(*sizing);
                                            }
                                        },
                                        None => (),
                                    };
                                }
                                
                                let oop_invested_new = if oop {
                                    *action_sizing
                                } else {
                                    current_node.oop_invested
                                };
                                
                                let ip_invested_new = if !oop {
                                    *action_sizing
                                } else {
                                    current_node.ip_invested
                                };
                                
                                let board_mask = get_card_mask(current_board);
                                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                                    let turn_board = &current_board[0..8];
                                    Some(get_card_mask(&turn_board))
                                } else {
                                    None
                                };
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(oop_new, actions_new, range_manager.get_num_hands(oop_new, board_mask, old_board_mask))), children: vec![], pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: oop_invested_new, ip_invested: ip_invested_new, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask) };
                                
                                recursive_build(Some(*action), sizing_mapping, &action_line, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                                
                            },
                            _ => panic!("Illegal action: Line: {} Current Action {:?}", action_line, action),
                        }
                    }
                    
                    
                },
                Some(ActionType::Check) => {
                    for action in actions {
                        match action {
                            ActionType::Check => { // XX line - terminal showdown (or to next chance node)
                                let node_type_new = if current_board.len() == 10 {
                                    NodeType::TerminalNode(TerminalType::TerminalShowdown)
                                } else {
                                    NodeType::ChanceNode((range_manager.get_board_deck(get_card_mask(current_board)).len() - 4).try_into().unwrap())
                                };
                                let board_mask = get_card_mask(current_board);
                                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                                    let turn_board = &current_board[0..8];
                                    Some(get_card_mask(&turn_board))
                                } else {
                                    None
                                };
                                let action_line = format!("{}:x",action_line);
                                let mut node_new = Node { node_type: node_type_new, children: vec![], pot_size: current_node.pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask) };
                                recursive_build(Some(*action), sizing_mapping, &action_line, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Bet(sizing) => { // XB line - add action node for OOP
                                let eff_stack_new = current_node.chance_start_stack - sizing;
                                let pot_size_new = current_node.pot_size + sizing;
                                let action_line = format!("{}:b{}",action_line,*sizing);
                                
                                let mut actions_new = vec![ActionType::Fold, ActionType::Call];
                                if eff_stack_new != 0 {                           
                                    match sizing_mapping.get(&action_line.to_owned()) {
                                        Some(action_sizings) => {
                                            for sizing in action_sizings {
                                                actions_new.push(*sizing);
                                            }
                                        },
                                        None => (),
                                    };
                                }
                                
                                let board_mask = get_card_mask(current_board);
                                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                                    let turn_board = &current_board[0..8];
                                    Some(get_card_mask(&turn_board))
                                } else {
                                    None
                                };
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(true, actions_new, range_manager.get_num_hands(true, board_mask, old_board_mask))), children: vec![], pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: *sizing, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask) };
                                recursive_build(Some(*action), sizing_mapping, &action_line, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            }
                            _ => panic!("Illegal action"),
                        }
                    }
                    
                },
                Some(ActionType::Raise{sizing}) => { 
                    // Look at our actions, create new nodes accordingly...
                    for action in actions {
                        match action {
                            ActionType::Fold => {
                                let eff_pot_size = current_node.pot_size - (sizing - min(current_node.oop_invested, current_node.ip_invested));
                                let board_mask = get_card_mask(current_board);
                                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                                    let turn_board = &current_board[0..8];
                                    Some(get_card_mask(&turn_board))
                                } else {
                                    None
                                };
                                let mut node_new = Node { node_type: NodeType::TerminalNode(TerminalType::TerminalFold(oop)), children: vec![], pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask) };
                                recursive_build(Some(*action), sizing_mapping, &action_line, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Call => {
                                let node_type_new = if current_board.len() == 10 {
                                    NodeType::TerminalNode(TerminalType::TerminalShowdown)
                                } else {
                                    NodeType::ChanceNode((range_manager.get_board_deck(get_card_mask(current_board)).len() - 4).try_into().unwrap())
                                };
                                let board_mask = get_card_mask(current_board);
                                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                                    let turn_board = &current_board[0..8];
                                    Some(get_card_mask(&turn_board))
                                } else {
                                    None
                                };
                                let action_line = format!("{}:c",action_line);
                                let eff_pot_size = current_node.pot_size + (sizing - min(current_node.oop_invested, current_node.ip_invested));
                                let mut node_new = Node { node_type: node_type_new, children: vec![], pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask) };
                                recursive_build(Some(*action), sizing_mapping, &action_line, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Raise{sizing: action_sizing} => {
                                let player_invested = match oop {
                                    true => current_node.oop_invested,
                                    false => current_node.ip_invested,
                                };
                                
                                let pot_size_new = current_node.pot_size + action_sizing - player_invested;
                                
                                let eff_stack_new = current_node.chance_start_stack - action_sizing;
                                let mut oop_new = oop;
                                oop_new ^= true;
                                let action_line = format!("{}:R{}",action_line,*action_sizing);
                                let mut actions_new = vec![ActionType::Fold, ActionType::Call];
                                
                                if eff_stack_new != 0 {
                                   match sizing_mapping.get(&action_line.to_owned()) {
                                        Some(action_sizings) => {
                                            for sizing in action_sizings {
                                                actions_new.push(*sizing);
                                            }
                                        },
                                        None => (),
                                    };
                                }
                                
                                let oop_invested_new = if oop {
                                    *action_sizing
                                } else {
                                    current_node.oop_invested
                                };
                                
                                let ip_invested_new = if !oop {
                                    *action_sizing
                                } else {
                                    current_node.ip_invested
                                };
                                
                                let board_mask = get_card_mask(current_board);
                                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                                    let turn_board = &current_board[0..8];
                                    Some(get_card_mask(&turn_board))
                                } else {
                                    None
                                };
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(oop_new, actions_new, range_manager.get_num_hands(oop_new, board_mask, old_board_mask))), children: vec![], pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: oop_invested_new, ip_invested: ip_invested_new, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask) };
                                recursive_build(Some(*action), sizing_mapping, &action_line, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            _ => panic!("Illegal action"),
                        }
                    }
                    
                },
                None => { // OOP's first decision
                    for action in actions {
                        match action {
                            ActionType::Check => {
                                let mut actions_new = vec![ActionType::Check];
                
                                
                                match sizing_mapping.get(&action_line.to_owned()) {
                                    Some(action_sizings) => {
                                        for sizing in action_sizings {
                                            actions_new.push(*sizing);
                                        }
                                    },
                                    None => (),
                                };
                               
                                
                                let board_mask = get_card_mask(current_board);
                                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                                    let turn_board = &current_board[0..8];
                                    Some(get_card_mask(&turn_board))
                                } else {
                                    None
                                };
                                let action_line = format!("{}:x",action_line);
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(false, actions_new, range_manager.get_num_hands(false, board_mask, old_board_mask))), children: vec![], pot_size: current_node.pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask)};
                                recursive_build(Some(*action), sizing_mapping, &action_line, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Bet(sizing) => {
                                let eff_stack_new = current_node.chance_start_stack - sizing;
                                let pot_size_new = current_node.pot_size + sizing;
                                let action_line = format!("{}:b{}",action_line,*sizing);
                                let mut actions_new = vec![ActionType::Fold, ActionType::Call];
                                if eff_stack_new != 0 {                           
                                    match sizing_mapping.get(&action_line.to_owned()) {
                                        Some(action_sizings) => {
                                            for sizing in action_sizings {
                                                actions_new.push(*sizing);
                                            }
                                        },
                                        None => (),
                                    };
                                }
                                
                                let board_mask = get_card_mask(current_board);
                                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                                    let turn_board = &current_board[0..8];
                                    Some(get_card_mask(&turn_board))
                                } else {
                                    None
                                };
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(false, actions_new, range_manager.get_num_hands(false, board_mask, old_board_mask))), children: vec![], pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: *sizing, ip_invested: 0, chance_start_pot: current_node.chance_start_pot, oop_num_hands: range_manager.get_num_hands(true, board_mask, old_board_mask), ip_num_hands: range_manager.get_num_hands(false, board_mask, old_board_mask) };
                                recursive_build(Some(*action), sizing_mapping, &action_line, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            _ => panic!("OOP made impossible first decision"),
                        }
                    }
                },
                //change this
                Some(_) => panic!("Invalid action"),
            }
        },
    }
}
