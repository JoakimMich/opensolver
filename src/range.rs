use rust_poker::hand_range::{get_card_mask};
use std::collections::HashMap;
use crate::hand_range::*;
use rust_poker::hand_evaluator::{Hand};

#[derive(Debug)]
pub struct RangeManager<'a> {
    pub oop_board_range: HashMap<u64, HandRange>,
    pub ip_board_range: HashMap<u64, HandRange>,
    pub initial_board: &'a str,
    pub oop_joint_combos: Vec<Option<usize>>,
    pub ip_joint_combos: Vec<Option<usize>>,
}

impl<'a> RangeManager<'a> {
    pub fn initialize_ranges(&mut self) {
        let board_mask = get_card_mask(self.initial_board);

        self.oop_board_range.get_mut(&board_mask).unwrap().remove_conflicting_combos(board_mask);
        self.ip_board_range.get_mut(&board_mask).unwrap().remove_conflicting_combos(board_mask);
        
        self.update_ranks();
        // TODO: Fix xxx_board_range for all possible turn/river runouts
        
        
    }
    
    pub fn new(oop_starting_hands: HandRange, ip_starting_hands: HandRange, initial_board: &str) -> RangeManager {
        let mut oop_board_range = HashMap::new();
        let mut ip_board_range = HashMap::new();
        let board_mask = get_card_mask(initial_board);
        oop_board_range.insert(board_mask, oop_starting_hands);
        ip_board_range.insert(board_mask, ip_starting_hands);
        let oop_joint_combos = vec![];
        let ip_joint_combos = vec![];
        
        RangeManager { oop_board_range, ip_board_range, initial_board, oop_joint_combos, ip_joint_combos }
    }
    
    // adds rank for all rivers and sorts accordingly
    pub fn update_ranks(&mut self) {
        for (key,value) in self.oop_board_range.iter_mut() {
            let community_cards = Hand::from_bit_mask(*key).count();
            if community_cards != 5 {
                continue;
            }
           
            for combo in value.hands.iter_mut() {
                combo.update_rank(*key);
            }
            value.sort_ranks();
        }
        
        for (key,value) in self.ip_board_range.iter_mut() {
            let community_cards = Hand::from_bit_mask(*key).count();
            if community_cards != 5 {
                continue;
            }
            
            for combo in value.hands.iter_mut() {
                combo.update_rank(*key);
                combo.update_joint(self.oop_board_range.get(key).unwrap());
            }
            value.sort_ranks();
        }
        
        for (key,value) in self.oop_board_range.iter_mut() {
            let community_cards = Hand::from_bit_mask(*key).count();
            if community_cards != 5 {
                continue;
            }
           
            for combo in value.hands.iter_mut() {
                combo.update_joint(self.ip_board_range.get(key).unwrap());
            }
        }
        
    }
    
    pub fn get_range(&self, oop: bool, board: &str) -> &HandRange {
        let board_mask = get_card_mask(board);
        
        match oop {
            true => self.oop_board_range.get(&board_mask).unwrap(),
            false => self.ip_board_range.get(&board_mask).unwrap(),
        }
    }
    
    pub fn get_num_hands(&self, oop: bool, board: &str) -> usize {
        let board_mask = get_card_mask(board);
        
        match oop {
            true => self.oop_board_range.get(&board_mask).unwrap().hands.len(),
            false => self.ip_board_range.get(&board_mask).unwrap().hands.len(),
        }
    }
    
    pub fn get_initial_reach_probs(&self, oop: bool) -> Vec<f64> {
        let board_mask = get_card_mask(self.initial_board);
        
        match oop {
            true => {
                let mut reach_probs = vec![0.0; self.oop_board_range.get(&board_mask).unwrap().hands.len()];
                
                for (i, reach_prob) in reach_probs.iter_mut().enumerate() {
                    *reach_prob = (self.oop_board_range.get(&board_mask).unwrap().hands[i].2) as f64 / 100.0
                }
                
                reach_probs
            },
            false => {
                let mut reach_probs = vec![0.0; self.ip_board_range.get(&board_mask).unwrap().hands.len()];
                
                for (i, reach_prob) in reach_probs.iter_mut().enumerate() {
                    *reach_prob = (self.ip_board_range.get(&board_mask).unwrap().hands[i].2) as f64 / 100.0
                }
                
                reach_probs
            }
        }
    }
    
    // TODO: initialize_reach_probs_mapping
}