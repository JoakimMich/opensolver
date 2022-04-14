use crate::range::*;
use std::cmp::min;
use rust_poker::constants::*;

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
        let mut offset = 0;
        let mut strategy = vec![0.0; self.hands_num * self.actions_num];
        for _ in 0..self.hands_num {
            let hand_sum_regrets: f64 = self.regret_sum[offset..offset+self.actions_num].iter().map(|x| (*x).max(0.0)).collect::<Vec<f64>>().iter().sum();
            if hand_sum_regrets > 0.0 {
                let pos_cum_regrets = &self.regret_sum[offset..offset+self.actions_num].iter().map(|x| (*x).max(0.0) / hand_sum_regrets).collect::<Vec<f64>>();
                for (count, val) in pos_cum_regrets.iter().enumerate() {
                    strategy[offset+count] = *val;
                }
            } else {
                for item in strategy.iter_mut().skip(offset).take(self.actions_num) {
                    *item = 1.0/self.actions_num as f64;
                }
            }
    
            offset += self.actions_num;
        }
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
    ChanceNodeCard(Option<u8>),
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
}

impl Node {
    pub fn new_root(chance_start_stack: u32, pot_size: u32) -> Node {
        Node { node_type: NodeType::ChanceNode(0), children: vec![] , pot_size, chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: pot_size}
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

pub struct SizingSchemes {
    pub oop_bet_sizings: Vec<u32>,
    pub ip_bet_sizings: Vec<u32>,
    pub oop_raise_sizings: Vec<u32>,
    pub ip_raise_sizings: Vec<u32>,
}


pub fn recursive_build(latest_action: Option<ActionType>, sizing_schemes: &SizingSchemes, current_node: &mut Node, range_manager: &RangeManager, current_board: &str) {
    match &current_node.node_type {
        NodeType::ChanceNode(_) => { 
            match latest_action  {
                Some(_) => {
                    if current_board.len() == 6 {
                        // Flop, add new turn cards
                    } else if current_board.len() == 8 {
                        for card in range_manager.get_board_deck(current_board).iter() {
                            let rank = RANK_TO_CHAR[usize::from(card >> 2)];
                            let suit = SUIT_TO_CHAR[usize::from(card & 3)];
                            let mut new_board = current_board.to_string();
                            new_board.push(rank);
                            new_board.push(suit);
                            let new_board = new_board.as_str();
                            
                            let new_eff_stack = current_node.chance_start_stack - (current_node.pot_size - current_node.chance_start_pot)/2;
                            
                            let mut node_new = Node { node_type: NodeType::ChanceNodeCard(Some(*card)), children: vec![], pot_size: current_node.pot_size, chance_start_stack: new_eff_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.pot_size };
                            recursive_build(None, sizing_schemes, &mut node_new, range_manager, new_board);
                            current_node.children.push(node_new);
                        }
                    } else {
                        panic!("Current board must be either length of flop or turn");
                    }
                },
                None => {
                        let mut node_new = Node { node_type: NodeType::ChanceNodeCard(None), children: vec![], pot_size: current_node.pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot };
                        recursive_build(None, sizing_schemes, &mut node_new, range_manager, current_board);
                        current_node.children.push(node_new);
                }
            };
        },
        NodeType::ChanceNodeCard(_) => {
            let mut actions_new = vec![];
            
            if current_node.chance_start_stack == 0 {
                actions_new.push(ActionType::Check);
            } else {
                for sizing in &sizing_schemes.oop_bet_sizings {
                    // TODO: Add -1 as sizing for allin???
                    match sizing {
                        sizing if sizing == &0 => actions_new.push(ActionType::Check),
                        _ => { 
                            if (*sizing as f64/100.0) * current_node.pot_size as f64 > current_node.chance_start_stack as f64 {
                                actions_new.push(ActionType::Bet(current_node.chance_start_stack));
                            } else if ((*sizing as f64/100.0) * current_node.pot_size as f64) as u32 != 0 {
                                actions_new.push(ActionType::Bet(((*sizing as f64/100.0) * current_node.pot_size as f64) as u32)); 
                            }
                        },
                    }
                }
            }
            actions_new.dedup();
            let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(true, actions_new, range_manager.get_num_hands(true, current_board))), children: vec![], pot_size: current_node.pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot };
            
            recursive_build(None, sizing_schemes, &mut node_new, range_manager, current_board);
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
                                let mut node_new = Node { node_type: NodeType::TerminalNode(TerminalType::TerminalFold(oop)), children: vec![], pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Call => {
                                // Add terminal call, or next street if turn/river
                                let eff_pot_size = current_node.pot_size + (sizing - min(current_node.oop_invested,current_node.ip_invested));
                                let node_type_new = if current_board.len() == 10 {
                                    NodeType::TerminalNode(TerminalType::TerminalShowdown)
                                } else {
                                    NodeType::ChanceNode((range_manager.get_board_deck(current_board).len() - 4).try_into().unwrap())
                                };
                                let mut node_new = Node { node_type: node_type_new, children: vec![], pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
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
                                
                                if eff_stack_new != 0 {
                                    
                                    let player_sizings = match oop_new {
                                        true => &sizing_schemes.oop_raise_sizings,
                                        false => &sizing_schemes.ip_raise_sizings,
                                    };
                                    
                                    for new_sizing in player_sizings {
                                        let raise_sizing = action_sizing + (((action_sizing+pot_size_new-sizing) as f32 *(*new_sizing as f32/100.0)) as u32) ;                                    
                                        if raise_sizing > eff_stack_new {
                                            match oop {
                                                true => actions_new.push(ActionType::Raise{sizing: current_node.chance_start_stack}),
                                                false => actions_new.push(ActionType::Raise{sizing: current_node.chance_start_stack}),
                                            };
                                        } else {
                                            match oop {
                                                true => actions_new.push(ActionType::Raise{sizing: raise_sizing}),
                                                false => actions_new.push(ActionType::Raise{sizing: raise_sizing}),
                                            };
                                        }
                                    }
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
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(oop_new, actions_new, range_manager.get_num_hands(oop_new, current_board))), children: vec![], pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: oop_invested_new, ip_invested: ip_invested_new, chance_start_pot: current_node.chance_start_pot };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                                
                            },
                            _ => panic!("Illegal action"),
                        }
                    }
                    
                    
                },
                // should always be IP?  add IP bet sizings etc
                Some(ActionType::Check) => {
                    for action in actions {
                        match action {
                            ActionType::Check => { // XX line - terminal showdown (or to next chance node)
                                let node_type_new = if current_board.len() == 10 {
                                    NodeType::TerminalNode(TerminalType::TerminalShowdown)
                                } else {
                                    NodeType::ChanceNode((range_manager.get_board_deck(current_board).len() - 4).try_into().unwrap())
                                };
                                let mut node_new = Node { node_type: node_type_new, children: vec![], pot_size: current_node.pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Bet(sizing) => { // XB line - add action node for OOP
                                let eff_stack_new = current_node.chance_start_stack - sizing;
                                let pot_size_new = current_node.pot_size + sizing;
                                
                                let mut actions_new = vec![ActionType::Fold, ActionType::Call];
                                if eff_stack_new != 0 {                           
                                    for new_sizing in &sizing_schemes.oop_raise_sizings {
                                        // TODO: possible additions to add allin flag in sizings, eg -1 (then match)
                                        // Calculate raise sizing
                                        let raise_sizing = sizing + (((sizing+pot_size_new) as f32 *(*new_sizing as f32/100.0)) as u32);
                                        if raise_sizing > eff_stack_new {
                                            actions_new.push(ActionType::Raise{sizing: current_node.chance_start_stack});
                                        } else {
                                            actions_new.push(ActionType::Raise{sizing: raise_sizing});
                                        }
                                            
                                    }
                                }
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(true, actions_new, range_manager.get_num_hands(true, current_board))), children: vec![], pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: *sizing, chance_start_pot: current_node.chance_start_pot };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            }
                            _ => panic!("Illegal action"),
                        }
                    }
                    
                }
                Some(ActionType::Raise{sizing}) => { 
                    // Look at our actions, create new nodes accordingly...
                    for action in actions {
                        match action {
                            ActionType::Fold => {
                                let eff_pot_size = current_node.pot_size - (sizing - min(current_node.oop_invested, current_node.ip_invested));
                                let mut node_new = Node { node_type: NodeType::TerminalNode(TerminalType::TerminalFold(oop)), children: vec![], pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Call => { // TODO: add chance nodes for turn flops
                                let node_type_new = if current_board.len() == 10 {
                                    NodeType::TerminalNode(TerminalType::TerminalShowdown)
                                } else {
                                    NodeType::ChanceNode((range_manager.get_board_deck(current_board).len() - 4).try_into().unwrap())
                                };
                                let eff_pot_size = current_node.pot_size + (sizing - min(current_node.oop_invested, current_node.ip_invested));
                                let mut node_new = Node { node_type: node_type_new, children: vec![], pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
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
                                let mut actions_new = vec![ActionType::Fold, ActionType::Call];
                                
                                if eff_stack_new != 0 {
                                    
                                    let player_sizings = match oop_new {
                                        true => &sizing_schemes.oop_raise_sizings,
                                        false => &sizing_schemes.ip_raise_sizings,
                                    };
                                    
                                    for new_sizing in player_sizings {
                                        let raise_sizing = action_sizing + (((action_sizing+pot_size_new-sizing) as f32 *(*new_sizing as f32/100.0)) as u32);
                                        if raise_sizing > eff_stack_new {
                                            match oop {
                                                true => actions_new.push(ActionType::Raise{sizing: current_node.chance_start_stack}),
                                                false => actions_new.push(ActionType::Raise{sizing: current_node.chance_start_stack}),
                                            };
                                        } else {
                                            match oop {
                                                true => actions_new.push(ActionType::Raise{sizing: raise_sizing}),
                                                false => actions_new.push(ActionType::Raise{sizing: raise_sizing}),
                                            };
                                        }
                                    }
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
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(oop_new, actions_new, range_manager.get_num_hands(oop_new, current_board))), children: vec![], pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: oop_invested_new, ip_invested: ip_invested_new, chance_start_pot: current_node.chance_start_pot };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            _ => panic!("Illegal action"),
                        }
                    }
                    
                },
                Some(_) => panic!("Illegal action"),
                None => { // OOP's first decision
                    for action in actions {
                        match action {
                            ActionType::Check => {
                                let mut actions_new = vec![];
                
                                if current_node.chance_start_stack == 0 {
                                    actions_new.push(ActionType::Check);
                                } else {
                                    for sizing in &sizing_schemes.ip_bet_sizings {
                                        // TODO: Add -1 as sizing for allin???
                                        match sizing {
                                            sizing if sizing == &0 => actions_new.push(ActionType::Check),
                                            _ => { 
                                                if (*sizing as f64/100.0) * current_node.pot_size as f64 > current_node.chance_start_stack as f64 {
                                                    actions_new.push(ActionType::Bet(current_node.chance_start_stack));
                                                } else if ((*sizing as f64/100.0) * current_node.pot_size as f64) as u32 != 0 {
                                                    actions_new.push(ActionType::Bet(((*sizing as f64/100.0) * current_node.pot_size as f64) as u32)); 
                                                }
                                            },
                                        }
                                    }
                                }
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(false, actions_new, range_manager.get_num_hands(false, current_board))), children: vec![], pot_size: current_node.pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: current_node.chance_start_pot};
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Bet(sizing) => {
                                let eff_stack_new = current_node.chance_start_stack - sizing;
                                let pot_size_new = current_node.pot_size + sizing;
                                
                                let mut actions_new = vec![ActionType::Fold, ActionType::Call];
                                if eff_stack_new != 0 {                           
                                    for new_sizing in &sizing_schemes.ip_raise_sizings {
                                        // TODO: possible additions to add allin flag in sizings, eg -1 (then match)
                                        // Calculate raise sizing
                                        let raise_sizing = sizing + (((sizing+pot_size_new) as f32 *(*new_sizing as f32/100.0)) as u32);
                                        if raise_sizing > eff_stack_new {
                                            actions_new.push(ActionType::Raise{sizing: current_node.chance_start_stack});
                                        } else {
                                            actions_new.push(ActionType::Raise{sizing: raise_sizing});
                                        }
                                            
                                    }
                                }
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(false, actions_new, range_manager.get_num_hands(false, current_board))), children: vec![], pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: *sizing, ip_invested: 0, chance_start_pot: current_node.chance_start_pot };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            _ => panic!("OOP made impossible first decision"),
                        }
                    }
                },
            }
        },
        
    }
    
}
