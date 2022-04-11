use crate::range::*;
use crate::postfloptree::*;
use crate::cfr::*;
use crate::hand_range::*;

pub struct BestResponse<'a> {
    range_manager: &'a RangeManager<'a>,
    pub oop_relative_probs: Vec<f64>,
    pub ip_relative_probs: Vec<f64>,
}

impl<'a> BestResponse<'a> {
    pub fn new(range_manager: &'a RangeManager) -> BestResponse<'a> {
        let board = range_manager.initial_board;
        let oop_hands = range_manager.get_num_hands(true, board);
        let ip_hands = range_manager.get_num_hands(false, board);
        let oop_relative_probs = vec![0.0; oop_hands];
        let ip_relative_probs = vec![0.0; ip_hands];
        BestResponse { range_manager, oop_relative_probs, ip_relative_probs }
    }
    
    pub fn get_best_response_ev(&mut self, pos: bool, root: &Node) -> f64 {
        let mut total_ev = 0.0;
        
        let villain_pos = pos ^ true;
        let board = self.range_manager.initial_board;
        let hero_hands = self.range_manager.get_num_hands(pos, board);
        let hero_range = &self.range_manager.get_range(pos, board).hands;
        let villain_range = &self.range_manager.get_range(villain_pos, board).hands;
        
        let relative_probs = match pos {
            true => &self.oop_relative_probs,
            false => &self.ip_relative_probs,
        };
        let villain_reach_probs = self.range_manager.get_initial_reach_probs(villain_pos);
        
        let mut ev_results = vec![];
        let mut new_br = BestResponseState::new(self.range_manager, &mut ev_results, root, pos, &villain_reach_probs, board);
        new_br.run();
        
        for i in 0..hero_hands {
            total_ev += ev_results[i] / get_unblocked_count(hero_range[i], villain_range) * relative_probs[i];
        }
        
        total_ev
    }
    
    
    
    pub fn set_relative_probablities(&mut self, pos: bool) {
        let villain_pos = pos ^ true;
        let board = self.range_manager.initial_board;
        let hero_hands = self.range_manager.get_num_hands(pos, board);
        let hero_range = &self.range_manager.get_range(pos, board).hands;
        let villain_range = &self.range_manager.get_range(villain_pos, board).hands;
        
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
        
        let exploitability = (oop_ev + ip_ev) / 2.0 / (root.pot_size as f64) * 100.0;
        
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
    range_manager: &'a RangeManager<'a>,
    result: &'a mut Vec<f64>,
    node: &'a Node,
    oop: bool,
    villain_reach_probs: &'a Vec<f64>,
    board: &'a str,
}

impl<'a> BestResponseState<'a> {
    fn new(range_manager: &'a RangeManager, result: &'a mut Vec<f64>, node: &'a Node, oop: bool, villain_reach_probs: &'a Vec<f64>, board: &'a str ) -> BestResponseState<'a> {
        BestResponseState { range_manager, result, node, oop, villain_reach_probs, board }
    }
    
    pub fn run(&mut self) {       
        match self.node.node_type {
            NodeType::TerminalNode(terminal_type) => {
                *self.result = get_payoffs(self.oop, self.range_manager, self.board, self.node, self.villain_reach_probs, &terminal_type);
            },
            NodeType::ChanceNode(_) => { 
                let mut new_br = BestResponseState::new(self.range_manager, self.result,  &self.node.children[0], self.oop, self.villain_reach_probs, self.board);
                new_br.run();
            }, 
            NodeType::ActionNode(ref node_info) => {
                let n_actions = node_info.actions_num;                
                if node_info.oop == self.oop {
                    let hero_hands = self.range_manager.get_num_hands(self.oop, self.board);
                
                    *self.result = vec![f64::MIN; hero_hands];
                    let mut results = vec![vec![0.0;hero_hands]; n_actions];
                    
                    for (count,child) in self.node.children.iter().enumerate() {
                        let mut new_br = BestResponseState::new(self.range_manager, &mut results[count], child, self.oop, self.villain_reach_probs, self.board);
                        new_br.run();
                    }
                    
                    for i in 0..hero_hands {
                        for results_j in results.iter() {
                            if results_j[i] > self.result[i] {
                                self.result[i] = results_j[i];
                            }
                        }
                    }
                    
                    
                } else {
                    let villain_pos = self.oop ^ true;
                    let average_strategy = node_info.get_average_strategy();
                    let hero_hands = self.range_manager.get_num_hands(self.oop, self.board);
                    let villain_hands = self.range_manager.get_num_hands(villain_pos, self.board);                    
                    *self.result = vec![0.0; hero_hands];
                    let mut results = vec![vec![0.0;hero_hands]; n_actions];
                    let mut new_villain_reach_probs = vec![vec![0.0; villain_hands]; n_actions];
                    
                    for (count,child) in self.node.children.iter().enumerate() {
                        let mut offset = 0;
                        for i in 0..villain_hands {
                            new_villain_reach_probs[count][i] = average_strategy[offset+count] * self.villain_reach_probs[i];
                            offset += n_actions;
                        }
                        
                        let mut new_br = BestResponseState::new(self.range_manager, &mut results[count], child, self.oop, &new_villain_reach_probs[count], self.board);
                        new_br.run();
                    }
                    
                    for i in 0..hero_hands {
                        for results_j in results.iter() {
                            self.result[i] += results_j[i];
                        }
                    }
                                        
                }
                
            },
        }
    }
}