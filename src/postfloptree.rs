use crate::range::*;
use crate::cards::*;
use crate::hand_range::Combo;
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
}

impl ActionNodeInfo {
    pub fn new(oop: bool, actions: Vec<ActionType>, hands_num: usize) -> ActionNodeInfo {
        let actions_num = actions.len();
        let strategy_sum = vec![0.0; hands_num * actions_num];
        let regret_sum = strategy_sum.clone();
        
        ActionNodeInfo { oop, actions, strategy_sum, regret_sum, actions_num }
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
        for (regrets, utility) in self.regret_sum.chunks_exact_mut(self.actions_num).zip(action_utilities) {
            regrets[n_action] += utility;
        }
    }

    pub fn update_regret_sum_2(&mut self, action_utilities: &[f64], n_iterations: u64) {
        let mut x = f64::powf(n_iterations as f64, ALPHA);
        x = x / (x + 1.0);

        for (regrets, utility) in self.regret_sum.chunks_exact_mut(self.actions_num).zip(action_utilities) {
            for regret in regrets {
                *regret -= utility;
                if *regret > 0.0 {
                    *regret *= x;
                } else {
                    *regret *= BETA;
                }
            }
        }
    }

    pub fn update_strategy_sum(&mut self, strategy: &[f64], reach_probs: &[f64], n_iterations: u64 ) {
        let x = f64::powf(n_iterations as f64 / (n_iterations as f64 + 1.0), GAMMA);
        let n = self.actions_num;
        for ((sums, hand_strategy), reach_prob) in self.strategy_sum.chunks_exact_mut(n).zip(strategy.chunks_exact(n)).zip(reach_probs) {
            for (sum, action_prob) in sums.iter_mut().zip(hand_strategy) {
                *sum += action_prob * reach_prob;
                *sum *= x;
            }
        }
    }

    pub fn get_average_strategy(&self) -> Vec<f64> {
        let mut average_strategy = self.strategy_sum.clone();

        for hand_strategy in average_strategy.chunks_exact_mut(self.actions_num) {
            let total: f64 = hand_strategy.iter().sum();
            if total > 0.0 {
                hand_strategy.iter_mut().for_each(|x| *x /= total);
            } else {
                hand_strategy.iter_mut().for_each(|x| *x = 1.0/self.actions_num as f64);
            }
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

/// Key of the ranges on a board in the RangeManager: (board, turn board if solving from the flop
/// and the board is a river)
type RangeKey = (u64, Option<u64>);

fn range_key(range_manager: &RangeManager, board: &str) -> RangeKey {
    let previous_board = if range_manager.initial_board.len() == 6 && board.len() == 10 {
        Some(get_card_mask(&board[0..8]))
    } else {
        None
    };
    (get_card_mask(board), previous_board)
}

/// Position of a combo in the UPI hand order
fn hand_order_index(hand: &Combo, hand_order_mapping: &HashMap<String, usize>) -> usize {
    let hand_str = hand.to_string();
    match hand_order_mapping.get(&hand_str) {
        Some(idx) => *idx,
        None => {
            let hand_reversed = format!("{}{}", &hand_str[2..], &hand_str[0..2]);
            *hand_order_mapping.get(&hand_reversed).unwrap()
        },
    }
}

/// Where a UPI line leads: the node and the chips invested along the way
struct LineState<'a> {
    node: &'a Node,
    board: String,
    oop_invested: u32,
    ip_invested: u32,
    start_pot: u32,
    previous_invested: u32,
}

impl Node {
    pub fn new_root(chance_start_stack: u32, pot_size: u32, oop_num_hands: usize, ip_num_hands: usize) -> Node {
        Node { node_type: NodeType::ChanceNode(0), children: vec![] , pot_size, chance_start_stack, oop_invested: 0, ip_invested: 0, chance_start_pot: pot_size, oop_num_hands, ip_num_hands}
    }

    /// A node on the same street as `self`, holding the ranges for `range_key`
    fn new_child(&self, node_type: NodeType, pot_size: u32, oop_invested: u32, ip_invested: u32, range_manager: &RangeManager, range_key: RangeKey) -> Node {
        Node {
            node_type,
            children: vec![],
            pot_size,
            chance_start_stack: self.chance_start_stack,
            oop_invested,
            ip_invested,
            chance_start_pot: self.chance_start_pot,
            oop_num_hands: range_manager.get_num_hands(true, range_key.0, range_key.1),
            ip_num_hands: range_manager.get_num_hands(false, range_key.0, range_key.1),
        }
    }

    // functions for UPI compatibility

    /// Follows a UPI line (e.g. "r:0:c:b94:QdKs") from the root. `on_action` is called for every
    /// action taken along the way, with the acting node, the index of the action and the board.
    fn walk_line<'a>(&'a self, line: &str, range_manager: &RangeManager, mut on_action: impl FnMut(&ActionNodeInfo, usize, &str)) -> LineState<'a> {
        let v = line.split(':').collect::<Vec<&str>>();

        //todo: better error handling
        if (v[0] != "r" || v.len() < 2 || v[1] != "0") && line != "r" {
            panic!("Invalid line input");
        }

        let mut state = LineState {
            node: if line == "r" { &self.children[0] } else { &self.children[0].children[0] },
            board: range_manager.initial_board.clone(),
            oop_invested: 0,
            ip_invested: 0,
            start_pot: self.children[0].pot_size,
            previous_invested: 0,
        };

        let mut latest_action = "";
        for &action in v.iter().skip(2) {
            if action == "f" || action == "c" || action.contains('b') {
                let facing_bet = latest_action.contains('b');
                // Chips the actor has invested on this street after the action, if it changes
                let (action_lookup, invested) = if action == "f" {
                    (ActionType::Fold, None)
                } else if action == "c" {
                    if facing_bet {
                        let call_sizing = latest_action[1..].parse::<u32>().unwrap() - state.previous_invested;
                        (ActionType::Call, Some(call_sizing))
                    } else {
                        (ActionType::Check, Some(0))
                    }
                } else {
                    let sizing = action[1..].parse::<u32>().unwrap() - state.previous_invested;
                    if facing_bet {
                        (ActionType::Raise{sizing}, Some(sizing))
                    } else {
                        (ActionType::Bet(sizing), Some(sizing))
                    }
                };

                let node_info = match &state.node.node_type {
                    NodeType::ActionNode(node_info) => node_info,
                    _ => panic!("Couldn't find root action node"),
                };
                //todo: better error handling
                let action_num = node_info.actions.iter().position(|a| *a == action_lookup).expect("Couldn't find line");
                on_action(node_info, action_num, &state.board);

                if let Some(sizing) = invested {
                    if node_info.oop {
                        state.oop_invested = sizing + state.previous_invested;
                    } else {
                        state.ip_invested = sizing + state.previous_invested;
                    }
                    if action == "c" {
                        state.previous_invested += sizing;
                    }
                }
                state.node = &state.node.children[action_num];
            } else {
                // todo: error handling for invalid input cards
                let new_board = format!("{}{}", state.board, &action[0..2]);
                let new_key = range_key(range_manager, &new_board);
                let card_node = state.node.children.iter().find(|child| match child.node_type {
                    NodeType::ChanceNodeCard(key) => key == new_key,
                    _ => false,
                });
                if let Some(card_node) = card_node {
                    state.node = &card_node.children[0];
                }
                state.board = new_board;
            }
            latest_action = action;
        }

        state
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
        match line.rsplit_once(':') {
            Some((parent_line, _)) => {
                self.get_children(parent_line.to_string(), range_manager)
                    .into_iter()
                    .find(|child| child.line == line)
                    .expect("Couldn't find line of node")
            },
            None => NodeInfo { line: "r".to_string(), node_type: "ROOT".to_string(), board: range_manager.initial_board.clone(), pot: (0, 0, self.children[0].pot_size), children_count: 1, flags: vec![] },
        }
    }

    pub fn get_children(&self, line: String, range_manager: &RangeManager) -> Vec<NodeInfo> {
        let state = self.walk_line(&line, range_manager, |_, _, _| {});
        let node = state.node;

        match &node.node_type {
            NodeType::ActionNode(node_info) => {
                let mut children_info = vec![];
                for (action, child) in node_info.actions.iter().zip(&node.children) {
                    let mut oop_invested = state.oop_invested;
                    let mut ip_invested = state.ip_invested;
                    // a call matches the larger investment
                    let called = oop_invested.max(ip_invested);

                    let (next_action, node_type, children_count) = match &child.node_type {
                        NodeType::ActionNode(child_info) => {
                            let next_action = match *action {
                                ActionType::Check => "c".to_string(),
                                ActionType::Call => {
                                    (oop_invested, ip_invested) = (called, called);
                                    "c".to_string()
                                },
                                ActionType::Bet(sizing) | ActionType::Raise{sizing} => {
                                    if node_info.oop {
                                        oop_invested = child.oop_invested + state.previous_invested;
                                    } else {
                                        ip_invested = child.ip_invested + state.previous_invested;
                                    }
                                    format!("b{}", sizing + state.previous_invested)
                                },
                                ActionType::Fold => "f".to_string(),
                            };
                            let node_type = if child_info.oop { "OOP_DEC" } else { "IP_DEC" };
                            (next_action, node_type, child.children.len() as u32)
                        },
                        NodeType::TerminalNode(TerminalType::TerminalFold(_)) => ("f".to_string(), "END_NODE", 0),
                        NodeType::TerminalNode(TerminalType::TerminalShowdown) => {
                            (oop_invested, ip_invested) = (called, called);
                            ("c".to_string(), "END_NODE", 0)
                        },
                        NodeType::ChanceNode(children_count) => {
                            (oop_invested, ip_invested) = (called, called);
                            ("c".to_string(), "SPLIT_NODE", *children_count as u32)
                        },
                        NodeType::ChanceNodeCard(_) => continue,
                    };

                    children_info.push(NodeInfo { line: format!("{}:{}", line, next_action), node_type: node_type.to_string(), board: state.board.clone(), pot: (oop_invested, ip_invested, state.start_pot), children_count, flags: vec![] });
                }
                children_info
            },
            NodeType::ChanceNode(_) => {
                let board_mask = get_card_mask(&state.board);
                node.children.iter().map(|child| {
                    let new_card = match child.node_type {
                        NodeType::ChanceNodeCard((new_board_mask, _)) => mask_to_string(new_board_mask & !board_mask),
                        _ => panic!("all children in chance node should be ChanceNodeCard"),
                    };
                    NodeInfo { line: format!("{}:{}", line, new_card), node_type: "OOP_DEC".to_string(), board: format!("{}{}", state.board, new_card), pot: (state.oop_invested, state.ip_invested, state.start_pot), children_count: child.children[0].children.len() as u32, flags: vec![] }
                }).collect()
            },
            NodeType::ChanceNodeCard(_) => {
                // root
                vec![NodeInfo { line: "r:0".to_string(), node_type: "OOP_DEC".to_string(), board: state.board, pot: (state.oop_invested, state.ip_invested, state.start_pot), children_count: node.children[0].children.len() as u32, flags: vec![] }]
            },
            NodeType::TerminalNode(_) => vec![],
        }
    }

    pub fn get_strategy(&self, line: String, range_manager: &RangeManager, hand_order_mapping: &HashMap<String, usize>) -> Vec<Vec<f64>> {
        let state = self.walk_line(&line, range_manager, |_, _, _| {});

        // todo: better error handling
        match &state.node.node_type {
            NodeType::ActionNode(node_info) => {
                let mut final_strategy = vec![vec![0.0; hand_order_mapping.len()]; node_info.actions_num];
                let key = range_key(range_manager, &state.board);
                let player_range = &range_manager.get_range(node_info.oop, key.0, key.1).hands;
                let average_strategy = node_info.get_average_strategy();
                for (hand, action_freqs) in player_range.iter().zip(average_strategy.chunks(node_info.actions_num)) {
                    let hand_idx = hand_order_index(hand, hand_order_mapping);
                    for (i, action_freq) in action_freqs.iter().enumerate() {
                        final_strategy[i][hand_idx] = *action_freq;
                    }
                }
                final_strategy
            },
            _ => panic!("incorrect path"),
        }
    }

    pub fn get_range(&self, oop: bool, line: String, range_manager: &RangeManager, hand_order_mapping: &HashMap<String, usize>) -> Vec<f64> {
        let mut final_range = vec![0.0; hand_order_mapping.len()];
        for hand in &range_manager.get_range(oop, get_card_mask(&range_manager.initial_board), None).hands {
            final_range[hand_order_index(hand, hand_order_mapping)] = hand.2 as f64 / 100.0;
        }

        self.walk_line(&line, range_manager, |node_info, action_num, board| {
            if node_info.oop != oop {
                return;
            }
            //todo: remove hands from final_range which are impossible due to blockers?
            let key = range_key(range_manager, board);
            let player_range = &range_manager.get_range(oop, key.0, key.1).hands;
            let average_strategy = node_info.get_average_strategy();
            for (hand, action_freqs) in player_range.iter().zip(average_strategy.chunks(node_info.actions_num)) {
                final_range[hand_order_index(hand, hand_order_mapping)] *= action_freqs[action_num];
            }
        });

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

/// Converts PIO style lines into the bet sizes available after each action sequence. A line lists
/// the chips each player has invested in total after every action, e.g. "0 94 260 260" is
/// check, bet 94, raise to 260, call. Sequences are keyed like "r:x:b94:R260".
pub fn get_sizings(lines: Vec<Vec<u32>>) -> HashMap<String, Vec<ActionType>> {
    let mut sizing_mapping: HashMap<String, Vec<ActionType>> = HashMap::new();

    for line in lines.iter() {
        let mut latest_invested = 0;
        let mut previous_invested = 0; // invested before the current street
        let mut node_line = "r".to_string();

        for &invested in line.iter() {
            let last_action = node_line.rsplit(':').next().unwrap();
            let facing_bet = last_action.contains('b') || last_action.contains('R');

            if invested == latest_invested {
                if invested != 0 && facing_bet {
                    node_line.push_str(":c");
                    previous_invested = invested;
                } else {
                    node_line.push_str(":x");
                }
            } else if invested != 0 {
                let sizing = invested - previous_invested;
                let action = if facing_bet {
                    ActionType::Raise{sizing}
                } else {
                    ActionType::Bet(sizing)
                };
                let sizings = sizing_mapping.entry(node_line.clone()).or_default();
                if !sizings.contains(&action) {
                    sizings.push(action);
                }
                node_line.push_str(&match action {
                    ActionType::Raise{sizing} => format!(":R{}", sizing),
                    _ => format!(":b{}", sizing),
                });
            }

            latest_invested = invested;
        }
    }

    sizing_mapping
}

fn with_sizings(mut actions: Vec<ActionType>, sizing_mapping: &HashMap<String, Vec<ActionType>>, action_line: &str) -> Vec<ActionType> {
    if let Some(sizings) = sizing_mapping.get(action_line) {
        actions.extend_from_slice(sizings);
    }
    actions
}

/// Node that follows once betting on the current street is closed
fn street_end(range_manager: &RangeManager, board: &str) -> NodeType {
    if board.len() == 10 {
        NodeType::TerminalNode(TerminalType::TerminalShowdown)
    } else {
        NodeType::ChanceNode((range_manager.get_board_deck(get_card_mask(board)).len() - 4).try_into().unwrap())
    }
}

/// Builds the game tree below `root` from the bet sizes in `sizing_mapping`
pub fn build_tree(root: &mut Node, sizing_mapping: &HashMap<String, Vec<ActionType>>, range_manager: &RangeManager) {
    let board = &range_manager.initial_board;
    let key = range_key(range_manager, board);
    let mut card_node = root.new_child(NodeType::ChanceNodeCard(key), root.pot_size, 0, 0, range_manager, key);
    recursive_build(sizing_mapping, "r", &mut card_node, range_manager, board);
    root.children.push(card_node);
}

fn recursive_build(sizing_mapping: &HashMap<String, Vec<ActionType>>, action_line: &str, current_node: &mut Node, range_manager: &RangeManager, current_board: &str) {
    match &current_node.node_type {
        NodeType::TerminalNode(_) => (),
        NodeType::ChanceNode(_) => {
            // Deal the next street
            if current_board.len() != 6 && current_board.len() != 8 {
                panic!("Current board must be either length of flop or turn");
            }
            let chance_start_stack = current_node.chance_start_stack - (current_node.pot_size - current_node.chance_start_pot)/2;
            for &card in range_manager.get_board_deck(get_card_mask(current_board)) {
                let new_board = format!("{}{}", current_board, mask_to_string(1u64 << card));
                let key = range_key(range_manager, &new_board);
                let mut card_node = Node {
                    node_type: NodeType::ChanceNodeCard(key),
                    children: vec![],
                    pot_size: current_node.pot_size,
                    chance_start_stack,
                    oop_invested: 0,
                    ip_invested: 0,
                    chance_start_pot: current_node.pot_size,
                    oop_num_hands: range_manager.get_num_hands(true, key.0, key.1),
                    ip_num_hands: range_manager.get_num_hands(false, key.0, key.1),
                };
                recursive_build(sizing_mapping, action_line, &mut card_node, range_manager, &new_board);
                current_node.children.push(card_node);
            }
        },
        NodeType::ChanceNodeCard(_) => {
            // OOP opens the street
            let key = range_key(range_manager, current_board);
            let actions = with_sizings(vec![ActionType::Check], sizing_mapping, action_line);
            let node_info = ActionNodeInfo::new(true, actions, range_manager.get_num_hands(true, key.0, key.1));
            let mut child = current_node.new_child(NodeType::ActionNode(node_info), current_node.pot_size, 0, 0, range_manager, key);
            recursive_build(sizing_mapping, action_line, &mut child, range_manager, current_board);
            current_node.children.push(child);
        },
        NodeType::ActionNode(node_info) => {
            let key = range_key(range_manager, current_board);
            let oop = node_info.oop;
            // Size of the bet (or raise) the acting player is facing
            let facing = current_node.oop_invested.abs_diff(current_node.ip_invested);
            let mut children = Vec::with_capacity(node_info.actions.len());

            for &action in &node_info.actions {
                let (child_line, node_type, pot_size, oop_invested, ip_invested) = match action {
                    ActionType::Fold => {
                        (action_line.to_string(), NodeType::TerminalNode(TerminalType::TerminalFold(oop)), current_node.pot_size - facing, 0, 0)
                    },
                    ActionType::Check if oop => {
                        // IP acts after OOP checks
                        let child_line = format!("{}:x", action_line);
                        let actions = with_sizings(vec![ActionType::Check], sizing_mapping, &child_line);
                        let node_info = ActionNodeInfo::new(false, actions, range_manager.get_num_hands(false, key.0, key.1));
                        (child_line, NodeType::ActionNode(node_info), current_node.pot_size, 0, 0)
                    },
                    ActionType::Check => {
                        (format!("{}:x", action_line), street_end(range_manager, current_board), current_node.pot_size, 0, 0)
                    },
                    ActionType::Call => {
                        (format!("{}:c", action_line), street_end(range_manager, current_board), current_node.pot_size + facing, 0, 0)
                    },
                    ActionType::Bet(sizing) | ActionType::Raise{sizing} => {
                        let child_line = match action {
                            ActionType::Bet(_) => format!("{}:b{}", action_line, sizing),
                            _ => format!("{}:R{}", action_line, sizing),
                        };
                        let mut actions = vec![ActionType::Fold, ActionType::Call];
                        if sizing != current_node.chance_start_stack {
                            // not all-in, so re-raises are possible
                            actions = with_sizings(actions, sizing_mapping, &child_line);
                        }
                        let node_info = ActionNodeInfo::new(!oop, actions, range_manager.get_num_hands(!oop, key.0, key.1));
                        let (oop_invested, ip_invested, player_invested) = if oop {
                            (sizing, current_node.ip_invested, current_node.oop_invested)
                        } else {
                            (current_node.oop_invested, sizing, current_node.ip_invested)
                        };
                        (child_line, NodeType::ActionNode(node_info), current_node.pot_size + sizing - player_invested, oop_invested, ip_invested)
                    },
                };

                let mut child = current_node.new_child(node_type, pot_size, oop_invested, ip_invested, range_manager, key);
                recursive_build(sizing_mapping, &child_line, &mut child, range_manager, current_board);
                children.push(child);
            }

            current_node.children = children;
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hand_range::HandRange;

    fn actions(node: &Node) -> &Vec<ActionType> {
        match &node.node_type {
            NodeType::ActionNode(node_info) => &node_info.actions,
            _ => panic!("not an action node"),
        }
    }

    #[test]
    fn sizings_from_lines() {
        let sizings = get_sizings(vec![vec![0, 0], vec![94, 260, 260], vec![94, 94], vec![0, 50, 50], vec![0, 0, 0, 30, 30]]);
        assert_eq!(sizings["r"], vec![ActionType::Bet(94)]);
        assert_eq!(sizings["r:b94"], vec![ActionType::Raise{sizing: 260}]);
        assert_eq!(sizings["r:x"], vec![ActionType::Bet(50)]);
        // flop checks through, IP bets the turn after OOP checks
        assert_eq!(sizings["r:x:x:x"], vec![ActionType::Bet(30)]);
        assert_eq!(sizings.len(), 4);
    }

    #[test]
    fn sizings_deduplicated() {
        let sizings = get_sizings(vec![vec![0, 50, 50], vec![0, 80, 80], vec![0, 50, 150, 150]]);
        assert_eq!(sizings["r:x"], vec![ActionType::Bet(50), ActionType::Bet(80)]);
    }

    #[test]
    fn ip_uses_own_sizes_after_check() {
        let mut range_manager = RangeManager::new(HandRange::from_string("AA".to_string()), HandRange::from_string("KK".to_string()), "2c7d9sTh3h".to_string());
        range_manager.initialize_ranges();
        let mut root = Node::new_root(300, 100, 6, 6);
        build_tree(&mut root, &get_sizings(vec![vec![0, 0], vec![30, 30], vec![0, 70, 70]]), &range_manager);

        let oop_node = &root.children[0].children[0];
        assert_eq!(actions(oop_node), &vec![ActionType::Check, ActionType::Bet(30)]);
        let ip_node = &oop_node.children[0];
        assert_eq!(actions(ip_node), &vec![ActionType::Check, ActionType::Bet(70)]);
        // IP bets 70 into 100 and OOP can fold or call
        let facing_bet = &ip_node.children[1];
        assert_eq!(actions(facing_bet), &vec![ActionType::Fold, ActionType::Call]);
        assert_eq!((facing_bet.pot_size, facing_bet.oop_invested, facing_bet.ip_invested), (170, 0, 70));
        assert_eq!(facing_bet.children[0].pot_size, 100);
        assert_eq!(facing_bet.children[1].pot_size, 240);
    }
}
