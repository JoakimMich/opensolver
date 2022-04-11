use crate::range::*;
use std::cmp::min;

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
                //for count in offset..offset+self.actions_num {
                //    strategy[count] = 1.0/self.actions_num as f64;
                //    println!("1: {}",strategy[count]);
                //}
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
        //for i in 0..self.hands_num {
        //    self.regret_sum[offset+n_action] += action_utilities[i];
        //    offset += self.actions_num;
        //}
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
    ChanceNode(f64),
}

#[derive(Debug)]
pub struct Node {
    pub node_type: NodeType,
    pub children: Vec<Node>,
    pub eff_stack: u32,
    pub pot_size: u32,
    pub chance_start_stack: u32,
    pub oop_invested: u32,
    pub ip_invested: u32,
}

impl Node {
    pub fn new_root(eff_stack: u32, pot_size: u32, chance_start_stack: u32) -> Node {
        Node { node_type: NodeType::ChanceNode(1.0), children: vec![] , eff_stack, pot_size, chance_start_stack, oop_invested: 0, ip_invested: 0}
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
            let mut actions_new = vec![];
            
            // TODO: Assert oop_bet_sizings not empty
            for sizing in &sizing_schemes.oop_bet_sizings {
                // TODO: Add -1 as sizing for allin???
                match sizing {
                    sizing if sizing == &0 => actions_new.push(ActionType::Check),
                    _ => { 
                        if (*sizing as f64/100.0) * current_node.pot_size as f64 > current_node.eff_stack as f64 {
                            actions_new.push(ActionType::Bet(current_node.eff_stack));
                        } else if ((*sizing as f64/100.0) * current_node.pot_size as f64) as u32 != 0 {
                            actions_new.push(ActionType::Bet(((*sizing as f64/100.0) * current_node.pot_size as f64) as u32)); 
                        }
                    },
                }
            }
            actions_new.dedup();
            
            let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(true, actions_new, range_manager.get_num_hands(true, current_board))), children: vec![], eff_stack: current_node.eff_stack, pot_size: current_node.pot_size, chance_start_stack: current_node.eff_stack, oop_invested: 0, ip_invested: 0 };
            
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
                                let mut node_new = Node { node_type: NodeType::TerminalNode(TerminalType::TerminalFold(oop)), children: vec![], eff_stack: current_node.eff_stack, pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0 };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Call => {
                                // Add terminal call, or next street if turn/river
                                let eff_pot_size = current_node.pot_size + (sizing - min(current_node.oop_invested,current_node.ip_invested));
                                let mut node_new = Node { node_type: NodeType::TerminalNode(TerminalType::TerminalShowdown), children: vec![], eff_stack: current_node.eff_stack, pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0 };
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
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(oop_new, actions_new, range_manager.get_num_hands(oop_new, current_board))), children: vec![], eff_stack: eff_stack_new, pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: oop_invested_new, ip_invested: ip_invested_new };
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
                                let mut node_new = Node { node_type: NodeType::TerminalNode(TerminalType::TerminalShowdown), children: vec![], eff_stack: current_node.eff_stack, pot_size: current_node.pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0 };
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
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(true, actions_new, range_manager.get_num_hands(true, current_board))), children: vec![], eff_stack: eff_stack_new, pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: *sizing };
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
                                let mut node_new = Node { node_type: NodeType::TerminalNode(TerminalType::TerminalFold(oop)), children: vec![], eff_stack: current_node.eff_stack, pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0 };
                                recursive_build(Some(*action), sizing_schemes, &mut node_new, range_manager, current_board);
                                current_node.children.push(node_new);
                            },
                            ActionType::Call => { // TODO: add chance nodes for turn flops
                                //let pot_size_new = current_node.pot_size + sizing;
                                let eff_pot_size = current_node.pot_size + (sizing - min(current_node.oop_invested, current_node.ip_invested));
                                let mut node_new = Node { node_type: NodeType::TerminalNode(TerminalType::TerminalShowdown), children: vec![], eff_stack: current_node.eff_stack, pot_size: eff_pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0 };
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
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(oop_new, actions_new, range_manager.get_num_hands(oop_new, current_board))), children: vec![], eff_stack: eff_stack_new, pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: oop_invested_new, ip_invested: ip_invested_new };
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
                
                                // TODO: Assert oop_bet_sizings not empty
                                for sizing in &sizing_schemes.ip_bet_sizings {
                                    // TODO: Add -1 as sizing for allin???
                                    match sizing {
                                        sizing if sizing == &0 => actions_new.push(ActionType::Check),
                                        _ => { 
                                            if (*sizing as f64/100.0) * current_node.pot_size as f64 > current_node.eff_stack as f64 {
                                                actions_new.push(ActionType::Bet(current_node.eff_stack));
                                            } else if ((*sizing as f64/100.0) * current_node.pot_size as f64) as u32 != 0 {
                                                actions_new.push(ActionType::Bet(((*sizing as f64/100.0) * current_node.pot_size as f64) as u32)); 
                                            }
                                        },
                                    }
                                }
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(false, actions_new, range_manager.get_num_hands(false, current_board))), children: vec![], eff_stack: current_node.eff_stack, pot_size: current_node.pot_size, chance_start_stack: current_node.chance_start_stack, oop_invested: 0, ip_invested: 0};
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
                                
                                let mut node_new = Node { node_type: NodeType::ActionNode(ActionNodeInfo::new(false, actions_new, range_manager.get_num_hands(false, current_board))), children: vec![], eff_stack: eff_stack_new, pot_size: pot_size_new, chance_start_stack: current_node.chance_start_stack, oop_invested: *sizing, ip_invested: 0 };
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
