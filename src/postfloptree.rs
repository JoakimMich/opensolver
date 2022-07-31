use crate::range::*;
use std::cmp::min;
use rust_poker::constants::*;
use rust_poker::hand_range::{get_card_mask,mask_to_string};
use std::collections::HashMap;
use std::fmt;

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

#[derive(Debug)]
pub struct NodeInfo {
    pub line: String,
    pub node_type: String,
    pub board: String,
    pub pot: (u32, u32, u32),
    pub children_count: u32,
    pub flags: Vec<String>,
}

impl fmt::Display for NodeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}",self.line)?;
        writeln!(f, "{}",self.node_type)?;
        writeln!(f, "{}",self.board)?;
        writeln!(f, "{} {} {}",self.pot.0,self.pot.1,self.pot.2)?;
        writeln!(f, "{} children",self.children_count)?;
        writeln!(f, "flags: ")?;
        for el in &self.flags {
             write!(f, "{} ", el)?;
        }
        Ok(())
    }
}

fn chunks(s: &str, length: usize) -> impl Iterator<Item=&str> {
    assert!(length > 0);
    let mut indices = s.char_indices().map(|(idx, _)| idx).peekable();
    
    std::iter::from_fn(move || {
        let start_idx = match indices.next() {
            Some(idx) => idx,
            None => return None,
        };
        for _ in 0..length - 1 {
            indices.next();
        }
        let end_idx = match indices.peek() {
            Some(idx) => *idx,
            None => s.bytes().len(),
        };
        Some(&s[start_idx..end_idx])
    })
}

impl Node {
    pub fn new_root(chance_start_stack: u32, pot_size: u32, oop_num_hands: usize, ip_num_hands: usize) -> Node {
        Node { node_type: NodeType::ChanceNode(0), children: vec![] , pot_size, chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: pot_size, oop_num_hands, ip_num_hands}
    }
    
    // functions for UPI compatibility
    
    fn find_node(&self, line: &String, range_manager: &RangeManager) -> (String, &Node, u32, u32, u32, u32) {
        let v = line.as_str().split(':').collect::<Vec<&str>>();
        let mut current_board = range_manager.initial_board.clone();
        
        //todo: better error handling
        if (v[0] != "r" || v.len() < 2 || v[1] != "0") && (line != "r"){
            panic!("Invalid line input");
        }
        
        let mut current_node = if line == "r" { 
            &self.children[0]
        } else {
            &self.children[0].children[0]
        };
        
        let mut oop_invested = 0;
        let mut ip_invested = 0;
        let mut previous_invested = 0;
        let start_pot = current_node.pot_size;
        
        if v.len() > 2 {
            let mut latest_action = "";
            for action in &v[2..] {
                if action.contains("b") {
                    let mut sizing: u32 = action[1..].parse().unwrap();
                    sizing -= previous_invested;
                    let action_lookup = if latest_action.contains("b") {
                        ActionType::Raise{sizing}
                    } else {
                        ActionType::Bet(sizing)
                    };
                    match &current_node.node_type {
                        NodeType::ActionNode(node_info) => {
                            let mut action_num = -1;
                            
                            for (i,action_node) in node_info.actions.iter().enumerate() {
                                if *action_node == action_lookup {
                                    current_node = &current_node.children[i as usize];
                                    action_num = i as i64;
                                }
                            }
                            
                            if action_num == -1 {
                                //todo: better error handling
                                panic!("Couldn't find line");
                            }
                            
                            if node_info.oop == true {
                                oop_invested = sizing + previous_invested;
                            } else {
                                ip_invested = sizing + previous_invested;
                            }
                        },
                        _ => panic!("Couldn't find root action node"),
                    };
                } else if action == &"c" {
                    // check if call or check, by checking latest action
                    let latest_sizing = if latest_action.contains("b") {
                        let mut sizing: u32 = latest_action[1..].parse().unwrap();
                        sizing -= previous_invested;
                        sizing
                    } else {
                        0
                    };
                    let action_lookup = if latest_action.contains("b") {
                        ActionType::Call
                    } else {
                        ActionType::Check
                    };
                    
                    match &current_node.node_type {
                        NodeType::ActionNode(node_info) => {
                            let mut action_num = -1;
                            
                            for (i,action_node) in node_info.actions.iter().enumerate() {
                                if *action_node == action_lookup {
                                    current_node = &current_node.children[i as usize];
                                    action_num = i as i64;
                                }
                            }
                            
                            if action_num == -1 {
                                //todo: better error handling
                                panic!("Couldn't find line");
                            }
                            
                            if node_info.oop == true {
                                oop_invested = latest_sizing + previous_invested;
                            } else {
                                ip_invested = latest_sizing + previous_invested;
                            }
                        },
                        _ => panic!("Couldn't find root action node"),
                    };
                    previous_invested += latest_sizing;
                } else {
                    // todo: error handling for invalid input cards
                    let mut new_board = current_board.clone();
                    let rank = action.chars().next().unwrap();
                    let suit = action.chars().nth(1).unwrap();
                    new_board.push(rank);
                    new_board.push(suit);
                    let new_board_mask = get_card_mask(&new_board);
                    let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 8 {
                        Some(get_card_mask(&current_board))
                    } else {
                        None
                    };
                    for (i, child) in current_node.children.iter().enumerate() {
                        let child_id = match child.node_type {
                            NodeType::ChanceNodeCard((new, old)) => {
                                if new == new_board_mask && old == old_board_mask {
                                    Some(i)
                                } else {
                                    None
                                }
                            },
                            _ => None,
                        };
                        
                        if let Some(x) = child_id {
                            current_node = &current_node.children[x].children[0];
                            break;
                        }
                    }
                    current_board = new_board;
                }
                latest_action = action;
            }
        }
        
        (current_board, current_node, oop_invested, ip_invested, start_pot, previous_invested)
    }
    
    pub fn get_line_freq(&self, line: String, range_manager: &RangeManager, hand_order_mapping: &HashMap<String, usize>) -> f64 {
        let mut line_freqs = vec![0.0, 0.0];
        for (i,&oop) in [false, true].iter().enumerate() {
            let start_range = self.get_range(oop, "r".to_string(), range_manager, hand_order_mapping);
            let final_range = self.get_range(oop, line.clone(), range_manager, hand_order_mapping);
            let start_range_sum: f64 = start_range.iter().sum();
            let mut line_freq = 0.0;
            
            for (j,weight) in final_range.iter().enumerate() {
                if weight > &0.0 {
                    let hand_freq = weight / start_range[j];
                    line_freq += hand_freq * (start_range[j]/start_range_sum)
                }
            }
            
            line_freqs[i] = line_freq
        }
        line_freqs[0]*line_freqs[1]
    }
    
    pub fn get_node(&self, line: String, range_manager: &RangeManager) -> NodeInfo {
        let v: Vec<&str> = line.rsplitn(2, ':').collect();
        if v.len() > 1 {
            let child_info = self.get_children(v[1].to_string(), range_manager);
            for child in child_info {
                if child.line == line {
                    return child;
                }
            }
        } else {
            let mut current_node = &self.children[0];
            let mut oop_invested = 0;
            let mut ip_invested = 0;
            let start_pot = current_node.pot_size;
            return NodeInfo { line: "r".to_string(), node_type: "ROOT".to_string(), board: range_manager.initial_board.clone(), pot: (oop_invested,ip_invested,start_pot), children_count: 1, flags: vec![] };
        }
        
        panic!("Couldn't find line of node");
    }
    
    pub fn get_children(&self, line: String, range_manager: &RangeManager) -> Vec<NodeInfo> {
        let mut children_info_vec = vec![];
        
        let (current_board, current_node, mut oop_invested, mut ip_invested, mut start_pot, previous_invested) = self.find_node(&line, range_manager);
        
        match &current_node.node_type {
            NodeType::ActionNode(node_info_current) => {
                for (i,child) in current_node.children.iter().enumerate() {
                    let child_info = match &child.node_type {
                        NodeType::ActionNode(node_info_child) => {
                            let mut new_line = line.clone();
                            new_line.push(':');
                            let next_action = match node_info_current.actions[i] {
                                ActionType::Check => {
                                    "c".to_string()
                                },
                                ActionType::Call => {
                                    if oop_invested > ip_invested {
                                        ip_invested = oop_invested;
                                    } else {
                                        oop_invested = ip_invested;
                                    }
                                    "c".to_string()
                                },
                                ActionType::Bet(sizing) => {
                                    if node_info_current.oop == true {
                                        oop_invested = child.oop_invested + previous_invested;
                                    } else {
                                        ip_invested = child.ip_invested + previous_invested;
                                    };
                                    format!("b{}",sizing+previous_invested)
                                },
                                ActionType::Raise{sizing} => {
                                    if node_info_current.oop == true {
                                        oop_invested = child.oop_invested + previous_invested;
                                    } else {
                                        ip_invested = child.ip_invested + previous_invested;
                                    }
                                    format!("b{}",sizing+previous_invested)
                                },
                                ActionType::Fold => {
                                    "f".to_string()
                                }
                            };
                            let node_type = if node_info_child.oop == true {
                                "OOP_DEC".to_string()
                            } else {
                                "IP_DEC".to_string()
                            };
                            new_line.push_str(&next_action);
                            let child_info = NodeInfo { line: new_line, node_type: node_type, board: current_board.clone(), pot: (oop_invested, ip_invested, start_pot), children_count: child.children.len() as u32, flags: vec![] };
                            children_info_vec.push(child_info);
                        },
                        NodeType::TerminalNode(terminal_type) => {
                            let mut new_line = line.clone();
                            new_line.push(':');
                            let next_action = match terminal_type {
                                TerminalType::TerminalFold(_) => {
                                    "f".to_string()
                                },
                                TerminalType::TerminalShowdown => {
                                    if oop_invested > ip_invested {
                                        ip_invested = oop_invested;
                                    } else {
                                        oop_invested = ip_invested;
                                    }
                                    "c".to_string()
                                },
                            };
                            new_line.push_str(&next_action);
                            let child_info = NodeInfo { line: new_line, node_type: "END_NODE".to_string(), board: current_board.clone(), pot: (oop_invested, ip_invested, start_pot), children_count: 0, flags: vec![] };
                            children_info_vec.push(child_info);
                        },
                        NodeType::ChanceNode(children_count) => {
                            if oop_invested > ip_invested {
                                ip_invested = oop_invested;
                            } else {
                                oop_invested = ip_invested;
                            }
                            let mut new_line = line.clone();
                            new_line.push(':');
                            new_line.push('c');
                            let child_info = NodeInfo { line: new_line, node_type: "SPLIT_NODE".to_string(), board: current_board.clone(), pot: (oop_invested, ip_invested, start_pot), children_count: *children_count as u32, flags: vec![] };
                            children_info_vec.push(child_info);
                        },
                        _ => (),
                    };
                }
            },
            NodeType::ChanceNode(_) => {
                let cards_old = chunks(&current_board, 2).collect::<Vec<&str>>();;
                for child in &current_node.children {
                    match child.node_type {
                        NodeType::ChanceNodeCard((board_mask, _)) => {
                            let new_board = mask_to_string(board_mask);
                            let cards_new = chunks(&new_board, 2).collect::<Vec<&str>>();;
                            let mut new_card = "";
                            for card in cards_new {
                                if cards_old.contains(&card) == false {
                                    new_card = card;
                                }
                            }
                            
                            let mut new_board = current_board.clone();
                            new_board.push_str(new_card);
                            let mut new_line = line.clone();
                            new_line.push(':');
                            new_line.push_str(new_card);
                            let child_info = NodeInfo { line: new_line, node_type: "OOP_DEC".to_string(), board: new_board, pot: (oop_invested, ip_invested, start_pot), children_count: child.children[0].children.len() as u32, flags: vec![] };
                            children_info_vec.push(child_info);
                        },
                        _ => panic!("all children in chance node should be ChanceNodeCard"),
                    };
                }
            },
            NodeType::ChanceNodeCard(board_mask) => {
                // root
                let child_info = NodeInfo { line: "r:0".to_string(), node_type: "OOP_DEC".to_string(), board: current_board, pot: (oop_invested, ip_invested, start_pot), children_count: current_node.children[0].children.len() as u32, flags: vec![] };
                children_info_vec.push(child_info);
            },
            _ => {
                println!("do nuufing, shoudlnt even be reached here???");
            },
        };
        
        children_info_vec
    }
    
    pub fn get_strategy(&self, line: String, range_manager: &RangeManager, hand_order_mapping: &HashMap<String, usize>) -> Vec<Vec<f64>> {
        let (current_board, current_node, mut oop_invested, mut ip_invested, mut start_pot, previous_invested) = self.find_node(&line, range_manager);
        
        // todo: better error handling
        match &current_node.node_type {
            NodeType::ActionNode(node_info) => {
                let mut final_strategy = vec![vec![0.0; hand_order_mapping.len()]; node_info.actions_num];
                let new_board_mask = get_card_mask(&current_board);
                let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 10 {
                    Some(get_card_mask(&current_board[0..8].to_string()))
                } else {
                    None
                };
                let player_range = &range_manager.get_range(node_info.oop, new_board_mask, old_board_mask).hands;
                let mut player_range_mapping = HashMap::new();
                for hand in player_range {
                    let hand_idx_temp = hand_order_mapping.get(&hand.to_string());
                    let hand_reversed = format!("{}{}", &hand.to_string()[2..], &hand.to_string()[0..2]);
                    let hand_idx = match hand_idx_temp {
                        Some(x) => x,
                        None => {
                            hand_order_mapping.get(&hand_reversed).unwrap()
                        },
                    };
                    player_range_mapping.insert((hand.0, hand.1), hand_idx);
                }
                let average_strategy = node_info.get_average_strategy();
                let mut counter = 0;
                average_strategy.chunks(node_info.actions_num).for_each(|slice| {
                    for (i, action_freq) in slice.iter().enumerate() {
                        final_strategy[i][**player_range_mapping.get(&(player_range[counter].0, player_range[counter].1)).unwrap() as usize] = *action_freq;
                    }
                    counter += 1;
                });
                final_strategy
            },
            _ => panic!("incorrect path"),
        }
    }
    
    pub fn get_range(&self, oop: bool, line: String, range_manager: &RangeManager, hand_order_mapping: &HashMap<String, usize>) -> Vec<f64> {
        let v = line.as_str().split(':').collect::<Vec<&str>>();
        let mut final_range = vec![0.0; hand_order_mapping.len()];
        let mut player_range_mapping = HashMap::new();
        let mut current_board = range_manager.initial_board.clone();
        let mut player_range = &range_manager.get_range(oop, get_card_mask(&range_manager.initial_board), None).hands;
        
        for hand in player_range {
            let hand_idx_temp = hand_order_mapping.get(&hand.to_string());
            let hand_reversed = format!("{}{}", &hand.to_string()[2..], &hand.to_string()[0..2]);
            let hand_idx = match hand_idx_temp {
                Some(x) => x,
                None => {
                    hand_order_mapping.get(&hand_reversed).unwrap()
                },
            };
            final_range[*hand_idx as usize] = hand.2  as f64 / 100.0;
            player_range_mapping.insert((hand.0, hand.1), hand_idx);
        }
        
        if (v[0] != "r" || v.len() < 2 || v[1] != "0") && line != "r" {
            panic!("Invalid line input");
        }
        
        if v.len() > 2 {
            let mut latest_action = "";
            let mut previous_invested = 0;
            let mut oop_invested = 0;
            let mut ip_invested = 0;
            let mut current_node = &self.children[0].children[0];
            for action in &v[2..] {
                if action.contains("b") {
                    let mut sizing: u32 = action[1..].parse().unwrap();
                    sizing -= previous_invested;
                    let action_lookup = if latest_action.contains("b") {
                        ActionType::Raise{sizing}
                    } else {
                        ActionType::Bet(sizing)
                    };
                    match &current_node.node_type {
                        NodeType::ActionNode(node_info) => {
                            let mut action_num = -1;
                            
                            for (i,action_node) in node_info.actions.iter().enumerate() {
                                if *action_node == action_lookup {
                                    current_node = &current_node.children[i as usize];
                                    action_num = i as i64;
                                }
                            }
                            
                            if action_num == -1 {
                                //todo: better error handling
                                panic!("Couldn't find line");
                            } else {
                                if node_info.oop == oop {
                                    let average_strategy = node_info.get_average_strategy();
                                    let mut counter = 0;
                                    average_strategy.chunks(node_info.actions_num).for_each(|slice| {
                                        let action_freq = slice[action_num as usize];
                                        final_range[**player_range_mapping.get(&(player_range[counter].0, player_range[counter].1)).unwrap() as usize] *= action_freq;
                                        counter += 1;
                                    });
                                }
                            }
                            
                            if node_info.oop == true {
                                oop_invested = sizing + previous_invested;
                            } else {
                                ip_invested = sizing + previous_invested;
                            }
                        },
                        _ => panic!("Couldn't find root action node"),
                    };
                } else if action == &"c" {
                    // check if call or check, by checking latest action
                    let latest_sizing = if latest_action.contains("b") {
                        let mut sizing: u32 = latest_action[1..].parse().unwrap();
                        sizing -= previous_invested;
                        sizing
                    } else {
                        0
                    };
                    let action_lookup = if latest_action.contains("b") {
                        ActionType::Call
                    } else {
                        ActionType::Check
                    };
                    
                    match &current_node.node_type {
                        NodeType::ActionNode(node_info) => {
                            let mut action_num = -1;
                            
                            for (i,action_node) in node_info.actions.iter().enumerate() {
                                if *action_node == action_lookup {
                                    current_node = &current_node.children[i as usize];
                                    action_num = i as i64;
                                }
                            }
                            
                            if action_num == -1 {
                                //todo: better error handling
                                panic!("Couldn't find line");
                            } else {
                                if node_info.oop == oop {
                                    let average_strategy = node_info.get_average_strategy();
                                    let mut counter = 0;
                                    average_strategy.chunks(node_info.actions_num).for_each(|slice| {
                                        let action_freq = slice[action_num as usize];
                                        final_range[**player_range_mapping.get(&(player_range[counter].0, player_range[counter].1)).unwrap() as usize] *= action_freq;
                                        counter += 1;
                                    });
                                }
                            }
                            
                            if node_info.oop == true {
                                oop_invested = latest_sizing + previous_invested;
                            } else {
                                ip_invested = latest_sizing + previous_invested;
                            }
                        },
                        _ => panic!("Couldn't find root action node"),
                    };
                    previous_invested += latest_sizing;
                } else if action == &"f" {
                    let action_lookup = ActionType::Fold;
                    match &current_node.node_type {
                        NodeType::ActionNode(node_info) => {
                            let mut action_num = -1;
                            
                            for (i,action_node) in node_info.actions.iter().enumerate() {
                                if *action_node == action_lookup {
                                    current_node = &current_node.children[i as usize];
                                    action_num = i as i64;
                                }
                            }
                            
                            if action_num == -1 {
                                //todo: better error handling
                                panic!("Couldn't find line");
                            } else {
                                if node_info.oop == oop {
                                    let average_strategy = node_info.get_average_strategy();
                                    let mut counter = 0;
                                    average_strategy.chunks(node_info.actions_num).for_each(|slice| {
                                        let action_freq = slice[action_num as usize];
                                        final_range[**player_range_mapping.get(&(player_range[counter].0, player_range[counter].1)).unwrap() as usize] *= action_freq;
                                        counter += 1;
                                    });
                                }
                            }
                        },
                        _ => panic!("Couldn't find root action node"),
                    };
                } else {
                    // todo: error handling for invalid input cards
                    let mut new_board = current_board.clone();
                    let rank = action.chars().next().unwrap();
                    let suit = action.chars().nth(1).unwrap();
                    new_board.push(rank);
                    new_board.push(suit);
                    let new_board_mask = get_card_mask(&new_board);
                    let old_board_mask = if range_manager.initial_board.len() == 6 && current_board.len() == 8 {
                        Some(get_card_mask(&current_board))
                    } else {
                        None
                    };
                    for (i, child) in current_node.children.iter().enumerate() {
                        let child_id = match child.node_type {
                            NodeType::ChanceNodeCard((new, old)) => {
                                if new == new_board_mask && old == old_board_mask {
                                    Some(i)
                                } else {
                                    None
                                }
                            },
                            _ => None,
                        };
                        
                        if let Some(x) = child_id {
                            current_node = &current_node.children[x].children[0];
                            break;
                        }
                    }
                    player_range = &range_manager.get_range(oop, new_board_mask, old_board_mask).hands;
                    for hand in player_range {
                        //todo: remove hands from final_range which are not impossible due to blockers?
                        let hand_idx_temp = hand_order_mapping.get(&hand.to_string());
                        let hand_reversed = format!("{}{}", &hand.to_string()[2..], &hand.to_string()[0..2]);
                        let hand_idx = match hand_idx_temp {
                            Some(x) => x,
                            None => {
                                hand_order_mapping.get(&hand_reversed).unwrap()
                            },
                        };
                        player_range_mapping.insert((hand.0, hand.1), hand_idx);
                    }
                    current_board = new_board;
                }
                latest_action = action;
            }
        }
        
        final_range
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
