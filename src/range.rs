use rust_poker::hand_range::{get_card_mask};
use std::collections::HashMap;
use crate::hand_range::*;
use rust_poker::hand_evaluator::{Hand};
use rust_poker::constants::RANK_TO_CHAR;
use rust_poker::constants::SUIT_TO_CHAR;
extern crate permutation;

#[derive(Debug)]
pub struct RangeManager<'a> {
    pub oop_board_range: HashMap<u64, HandRange>,
    pub ip_board_range: HashMap<u64, HandRange>,
    pub initial_board: &'a str,
    pub oop_joint_combos: Vec<Option<usize>>,
    pub ip_joint_combos: Vec<Option<usize>>,
    pub board_deck: HashMap<u64, Vec<u8>>,
    oop_reach_mapping: HashMap<u64, Vec<u16>>,
    ip_reach_mapping: HashMap<u64, Vec<u16>>,
}

fn board_to_u8(board: &str) -> Vec<u8> {
    let mut u8_vec: Vec<u8> = vec![];
    
    for i in (0..board.len()).step_by(2) {
        let card = &board[i..i+2].to_lowercase().to_string();
        let rank = card.chars().next().unwrap();
        let suit = card.chars().nth(1).unwrap();
        let card_nr = char_to_rank(rank)*4+char_to_suit(suit);
        u8_vec.push(card_nr);
    }
    
    u8_vec
}

fn u8_to_board(u8_board: Vec<u8>) -> String {
    let mut board = String::new();
    
    for card in u8_board {
        let rank = RANK_TO_CHAR[usize::from(card >> 2)];
        let suit = SUIT_TO_CHAR[usize::from(card & 3)];
        board.push(rank);
        board.push(suit);
    }
    
    board
}

impl<'a> RangeManager<'a> {
    pub fn initialize_ranges(&mut self) {
        let board_mask = get_card_mask(self.initial_board);

        self.oop_board_range.get_mut(&board_mask).unwrap().remove_conflicting_combos(board_mask);
        self.ip_board_range.get_mut(&board_mask).unwrap().remove_conflicting_combos(board_mask);
        
        if self.initial_board.len() == 6 {
            
        } else if self.initial_board.len() == 8 {
            let river_cards = self.get_board_deck(self.initial_board).clone();
            for card in river_cards.iter() {
                let rank = RANK_TO_CHAR[usize::from(card >> 2)];
                let suit = SUIT_TO_CHAR[usize::from(card & 3)];
                let mut new_board = self.initial_board.to_string();
                new_board.push(rank);
                new_board.push(suit);
                let new_board = new_board.as_str();
                let new_board_mask = get_card_mask(new_board);
                self.oop_board_range.insert(new_board_mask, self.oop_board_range.get(&board_mask).unwrap().clone());
                self.ip_board_range.insert(new_board_mask, self.ip_board_range.get(&board_mask).unwrap().clone());
                self.oop_board_range.get_mut(&new_board_mask).unwrap().remove_conflicting_combos(new_board_mask);
                self.ip_board_range.get_mut(&new_board_mask).unwrap().remove_conflicting_combos(new_board_mask);
            }
        } else if self.initial_board.len() == 10 {
            
        } else {
            panic!("Initial board invalid length");
        }
                
        self.update_ranks();
        self.update_joints();
    }
    
    pub fn new(oop_starting_hands: HandRange, ip_starting_hands: HandRange, initial_board: &str) -> RangeManager {
        let mut oop_board_range = HashMap::new();
        let mut ip_board_range = HashMap::new();
        let oop_reach_mapping = HashMap::new();
        let ip_reach_mapping = HashMap::new();
        let mut board_deck = HashMap::new();
        if initial_board.len() != 6 {
            let board_mask = get_card_mask(initial_board);
            oop_board_range.insert(board_mask, oop_starting_hands);
            ip_board_range.insert(board_mask, ip_starting_hands);
            if initial_board.len() == 8 {
                let card_deck_left: Vec<u8> = (0..52).collect();
                let board_u8 = board_to_u8(initial_board);
                let card_deck_left: Vec<u8> = card_deck_left.iter().filter(|&x| !board_u8.contains(x)).cloned().collect();
                board_deck.insert(board_mask, card_deck_left);
            }
            
        } else {
            // do isomorphic stuff (normalize init board, add only neccesary suits)
        }
        let oop_joint_combos = vec![];
        let ip_joint_combos = vec![];
        
        RangeManager { oop_board_range, ip_board_range, initial_board, oop_joint_combos, ip_joint_combos, board_deck, oop_reach_mapping, ip_reach_mapping }
    }
    
    pub fn get_board_deck(&self, board: &str) -> &Vec<u8> {
        let board_mask = get_card_mask(board);
        self.board_deck.get(&board_mask).unwrap()
    }
    
    pub fn get_reach_mapping(&self, oop: bool, board: &str) -> &Vec<u16> {
        let board_mask = get_card_mask(board);
        match oop {
            true => self.oop_reach_mapping.get(&board_mask).unwrap(),
            false => self.ip_reach_mapping.get(&board_mask).unwrap(),
        }
    }
    
    pub fn get_villain_reach(&self, oop: bool, board: &str, current_reach: &[f64]) -> Vec<f64> {
        let villain_pos = oop ^ true;
        let reach_mapping = self.get_reach_mapping(villain_pos, board);
        let mut new_reach = vec![0.0; reach_mapping.len()];
        
        for (count,reach) in reach_mapping.iter().enumerate() {
            new_reach[count] = current_reach[*reach as usize];
        }
        
        new_reach
    }
    
    pub fn update_joints(&mut self) {
        for (key,value) in self.oop_board_range.iter_mut() {
            let community_cards = Hand::from_bit_mask(*key).count();
            if community_cards != 5 {
                continue;
            }
           
            for combo in value.hands.iter_mut() {
                combo.update_joint(self.ip_board_range.get(key).unwrap());
            }
        }
        
        for (key,value) in self.ip_board_range.iter_mut() {
            let community_cards = Hand::from_bit_mask(*key).count();
            if community_cards != 5 {
                continue;
            }
           
            for combo in value.hands.iter_mut() {
                combo.update_joint(self.oop_board_range.get(key).unwrap());
            }
        }
    }
    
    // adds rank for all rivers and sorts accordingly
    pub fn update_ranks(&mut self) {
        let oop_hashmap = self.oop_board_range.clone();
        let ip_hashmap = self.ip_board_range.clone();
        
        for (key,value) in self.oop_board_range.iter_mut() {
            let community_cards = Hand::from_bit_mask(*key).count();
            
            if community_cards < 4 {
                continue;
            }
            
            if self.initial_board.len() == 10 {
                for combo in value.hands.iter_mut() {
                    combo.update_rank(*key);
                }
                
                let hand_range = value.clone();
                let permutation = permutation::sort_by_key(&hand_range.hands, |k| k.3);
                let hand_range = permutation.apply_slice(&hand_range.hands);
                value.hands = hand_range;
                continue;
            } else if self.initial_board.len() == 8 && community_cards == 4 {
                continue;
            }
            
            // Turns when solved from flop
            if community_cards == 4 && self.initial_board.len() != 8 {
                todo!();
            } 
            
            // Rivers when solved from flop or turn 
            
            let turn_range = if self.initial_board.len() == 6 {
                let river = mask_to_string(*key);
                let river_vec = board_to_u8(&river);
                let initial_vec = board_to_u8(self.initial_board);
                let common_vec: Vec<u8> = river_vec.iter().filter(|&x| initial_vec.contains(x)).cloned().collect();
                let turn = u8_to_board(common_vec);
                let turn_mask = get_card_mask(turn.as_str());
                &oop_hashmap.get(&turn_mask).unwrap().hands
            } else {
                &oop_hashmap.get(&get_card_mask(self.initial_board)).unwrap().hands
            };
            
            let mut j = 0;
            let mut reach_probs = vec![0; value.hands.len()];
            for (count, combo) in value.hands.iter().enumerate() {
                while *combo != turn_range[j] {
                    j += 1;
                }
                reach_probs[count] = j as u16;
                
            }
            
            for combo in value.hands.iter_mut() {
                combo.update_rank(*key);
            }
            
            let hand_range = value.clone();
            let permutation = permutation::sort_by_key(&hand_range.hands, |k| k.3);
            let hand_range = permutation.apply_slice(&hand_range.hands);
            value.hands = hand_range;
            let new_reach_probs = permutation.apply_slice(reach_probs);
            
            self.oop_reach_mapping.insert(*key, new_reach_probs);
        }
        
        for (key,value) in self.ip_board_range.iter_mut() {
            let community_cards = Hand::from_bit_mask(*key).count();
            
            if community_cards < 4 {
                continue;
            }
            
            if self.initial_board.len() == 10 {
                for combo in value.hands.iter_mut() {
                    combo.update_rank(*key);
                }
                
                let hand_range = value.clone();
                let permutation = permutation::sort_by_key(&hand_range.hands, |k| k.3);
                let hand_range = permutation.apply_slice(&hand_range.hands);
                value.hands = hand_range;
                continue;
            } else if self.initial_board.len() == 8 && community_cards == 4 {
                continue;
            }
            
            // Turns when solved from flop
            if community_cards == 4 && self.initial_board.len() != 8 {
                todo!();
            } 
            
            // Rivers when solved from flop or turn 
            
            let turn_range = if self.initial_board.len() == 6 {
                let river = mask_to_string(*key);
                let river_vec = board_to_u8(&river);
                let initial_vec = board_to_u8(self.initial_board);
                let common_vec: Vec<u8> = river_vec.iter().filter(|&x| initial_vec.contains(x)).cloned().collect();
                let turn = u8_to_board(common_vec);
                let turn_mask = get_card_mask(turn.as_str());
                &ip_hashmap.get(&turn_mask).unwrap().hands
            } else {
                &ip_hashmap.get(&get_card_mask(self.initial_board)).unwrap().hands
            };
            
            let mut j = 0;
            let mut reach_probs = vec![0; value.hands.len()];
            for (count, combo) in value.hands.iter().enumerate() {
                while *combo != turn_range[j] {
                    j += 1;
                }
                reach_probs[count] = j as u16;
                
            }
            
            for combo in value.hands.iter_mut() {
                combo.update_rank(*key);
            }
            
            let hand_range = value.clone();
            let permutation = permutation::sort_by_key(&hand_range.hands, |k| k.3);
            let hand_range = permutation.apply_slice(&hand_range.hands);
            value.hands = hand_range;
            let new_reach_probs = permutation.apply_slice(reach_probs);
            
            self.ip_reach_mapping.insert(*key, new_reach_probs);
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