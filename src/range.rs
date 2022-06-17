use rust_poker::hand_range::{get_card_mask};
use std::collections::HashMap;

use crate::hand_range::*;
use crate::isomorphism::*;

use rust_poker::hand_evaluator::{Hand};
use rust_poker::constants::RANK_TO_CHAR;
use rust_poker::constants::SUIT_TO_CHAR;

extern crate permutation;

#[derive(Debug)]
pub struct RangeManager {
    pub oop_board_range: HashMap<(u64, Option<u64>), HandRange>,
    pub ip_board_range: HashMap<(u64, Option<u64>), HandRange>,
    pub initial_board: String,
    pub oop_joint_combos: Vec<Option<usize>>,
    pub ip_joint_combos: Vec<Option<usize>>,
    pub board_deck: HashMap<u64, Vec<u8>>,
    oop_reach_mapping: HashMap<(u64, Option<u64>), Vec<u16>>,
    ip_reach_mapping: HashMap<(u64, Option<u64>), Vec<u16>>,
}

fn board_to_u8(board: &String) -> Vec<u8> {
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

impl RangeManager {
    pub fn initialize_ranges(&mut self) {
        let board_mask = get_card_mask(&self.initial_board);

        self.oop_board_range.get_mut(&(board_mask, None)).unwrap().remove_conflicting_combos(board_mask);
        self.ip_board_range.get_mut(&(board_mask, None)).unwrap().remove_conflicting_combos(board_mask);
        
        if self.initial_board.len() == 6 {
            let turn_cards = self.get_board_deck(board_mask).clone();
            for card in turn_cards.iter() {
                let rank = RANK_TO_CHAR[usize::from(card >> 2)];
                let suit = SUIT_TO_CHAR[usize::from(card & 3)];
                let mut new_board = self.initial_board.clone();
                new_board.push(rank);
                new_board.push(suit);
                let new_board_mask = get_card_mask(&new_board);
                self.oop_board_range.insert((new_board_mask, None), self.oop_board_range.get(&(board_mask, None)).unwrap().clone());
                self.ip_board_range.insert((new_board_mask, None), self.ip_board_range.get(&(board_mask, None)).unwrap().clone());
                self.oop_board_range.get_mut(&(new_board_mask, None)).unwrap().remove_isomorphic(&new_board);
                self.oop_board_range.get_mut(&(new_board_mask, None)).unwrap().remove_conflicting_combos(new_board_mask);
                self.ip_board_range.get_mut(&(new_board_mask, None)).unwrap().remove_isomorphic(&new_board);
                self.ip_board_range.get_mut(&(new_board_mask, None)).unwrap().remove_conflicting_combos(new_board_mask);
                
                let river_cards = self.get_board_deck(new_board_mask).clone();
                for river_card in river_cards.iter() {
                    let rank = RANK_TO_CHAR[usize::from(river_card >> 2)];
                    let suit = SUIT_TO_CHAR[usize::from(river_card & 3)];
                    let mut new_board = new_board.clone();
                    new_board.push(rank);
                    new_board.push(suit);
                    let river_board_mask = get_card_mask(&new_board);
                    self.oop_board_range.insert((river_board_mask, Some(new_board_mask)), self.oop_board_range.get(&(new_board_mask, None)).unwrap().clone());
                    self.ip_board_range.insert((river_board_mask, Some(new_board_mask)), self.ip_board_range.get(&(new_board_mask, None)).unwrap().clone());
                    self.oop_board_range.get_mut(&(river_board_mask, Some(new_board_mask))).unwrap().remove_isomorphic(&new_board);
                    self.oop_board_range.get_mut(&(river_board_mask, Some(new_board_mask))).unwrap().remove_conflicting_combos(river_board_mask);
                    self.ip_board_range.get_mut(&(river_board_mask, Some(new_board_mask))).unwrap().remove_isomorphic(&new_board);
                    self.ip_board_range.get_mut(&(river_board_mask, Some(new_board_mask))).unwrap().remove_conflicting_combos(river_board_mask);
                }
            }
            
        } else if self.initial_board.len() == 8 {
            let river_cards = self.get_board_deck(board_mask).clone();
            for card in river_cards.iter() {
                let rank = RANK_TO_CHAR[usize::from(card >> 2)];
                let suit = SUIT_TO_CHAR[usize::from(card & 3)];
                let mut new_board = self.initial_board.clone();
                new_board.push(rank);
                new_board.push(suit);
                let new_board_mask = get_card_mask(&new_board);
                self.oop_board_range.insert((new_board_mask, None), self.oop_board_range.get(&(board_mask, None)).unwrap().clone());
                self.ip_board_range.insert((new_board_mask, None), self.ip_board_range.get(&(board_mask, None)).unwrap().clone());
                self.oop_board_range.get_mut(&(new_board_mask, None)).unwrap().remove_conflicting_combos(new_board_mask);
                self.ip_board_range.get_mut(&(new_board_mask, None)).unwrap().remove_conflicting_combos(new_board_mask);
            }
        } else if self.initial_board.len() == 10 {
            
        } else {
            panic!("Initial board invalid length");
        }
                
        self.update_ranks();
        self.update_joints();
    }
    
    pub fn new(mut oop_starting_hands: HandRange, mut ip_starting_hands: HandRange, initial_board: String) -> RangeManager {
        let mut oop_board_range = HashMap::new();
        let mut ip_board_range = HashMap::new();
        let oop_reach_mapping = HashMap::new();
        let ip_reach_mapping = HashMap::new();
        let mut board_deck = HashMap::new();
        let initial_board = if initial_board.len() == 6 {
            normalize_flop(&initial_board)
        } else {
            initial_board
        };
        
        if initial_board.len() != 6 {
            let board_mask = get_card_mask(&initial_board);
            oop_board_range.insert((board_mask, None), oop_starting_hands);
            ip_board_range.insert((board_mask, None), ip_starting_hands);
            if initial_board.len() == 8 {
                let card_deck_left: Vec<u8> = (0..52).collect();
                let board_u8 = board_to_u8(&initial_board);
                let card_deck_left: Vec<u8> = card_deck_left.iter().filter(|&x| !board_u8.contains(x)).cloned().collect();
                board_deck.insert(board_mask, card_deck_left);
            }
            
        } else {
            // do isomorphic stuff (normalize init board, add only neccesary suits)
            let board_mask = get_card_mask(&initial_board);
            oop_starting_hands.remove_isomorphic(&initial_board);
            ip_starting_hands.remove_isomorphic(&initial_board);
            oop_board_range.insert((board_mask, None), oop_starting_hands);
            ip_board_range.insert((board_mask, None), ip_starting_hands);
            
            let mut suits_remove = vec![];
            let iso_mapping = isomorphism_mapping(&initial_board);
            
            for (from_suit, to_suit) in &iso_mapping {
                if to_suit.is_lowercase() && (from_suit != to_suit) {
                    suits_remove.push(*from_suit);
                }
            }
            
            let card_deck_left: Vec<u8> = (0..52).collect();
            let board_u8 = board_to_u8(&initial_board);
            let mut card_deck_left: Vec<u8> = card_deck_left.iter().filter(|&x| !board_u8.contains(x)).cloned().collect();
            card_deck_left.retain(|x| suits_remove.contains(&SUIT_TO_CHAR[usize::from(x & 3)]) == false);
            
            let card_deck_left_copy = card_deck_left.clone();
            board_deck.insert(board_mask, card_deck_left);
            
            for u8_card in card_deck_left_copy.iter() {
                let rank = RANK_TO_CHAR[usize::from(u8_card >> 2)];
                let suit = SUIT_TO_CHAR[usize::from(u8_card & 3)];
                let mut new_board = initial_board.clone();
                new_board.push(rank);
                new_board.push(suit);
                let board_mask = get_card_mask(&new_board);
                
                let mut suits_remove = vec![];
                let iso_mapping = isomorphism_mapping(&new_board);
                
                for (from_suit, to_suit) in &iso_mapping {
                    if to_suit.is_lowercase() && (from_suit != to_suit) {
                        suits_remove.push(*from_suit);
                    }
                }
                
                let card_deck_left: Vec<u8> = (0..52).collect();
                let board_u8 = board_to_u8(&new_board);
                let mut card_deck_left: Vec<u8> = card_deck_left.iter().filter(|&x| !board_u8.contains(x)).cloned().collect();
                card_deck_left.retain(|x| suits_remove.contains(&SUIT_TO_CHAR[usize::from(x & 3)]) == false);
                board_deck.insert(board_mask, card_deck_left);
                
            }
        }
        let oop_joint_combos = vec![];
        let ip_joint_combos = vec![];
        
        RangeManager { oop_board_range, ip_board_range, initial_board, oop_joint_combos, ip_joint_combos, board_deck, oop_reach_mapping, ip_reach_mapping }
    }
    
    pub fn get_board_deck(&self, board: u64) -> &Vec<u8> {
        self.board_deck.get(&board).unwrap()
    }
    
    pub fn get_reach_mapping(&self, oop: bool, board: u64, previous_board: Option<u64>) -> &Vec<u16> {
        match oop {
            true => self.oop_reach_mapping.get(&(board, previous_board)).unwrap(),
            false => self.ip_reach_mapping.get(&(board, previous_board)).unwrap(),
        }
    }
    
    pub fn get_villain_reach(&self, oop: bool, board: u64, previous_board: Option<u64>, current_reach: &[f64]) -> Vec<f64> {
        let villain_pos = oop ^ true;
        let reach_mapping = self.get_reach_mapping(villain_pos, board, previous_board);
        let mut new_reach = vec![0.0; reach_mapping.len()];
        for (count,reach) in reach_mapping.iter().enumerate() {
            new_reach[count] = current_reach[*reach as usize];
        }
        
        new_reach
    }
    
    pub fn update_joints(&mut self) {
        for (key,value) in self.oop_board_range.iter_mut() {
            let community_cards = Hand::from_bit_mask(key.0).count();
            if community_cards != 5 {
                continue;
            }
           
            for combo in value.hands.iter_mut() {
                combo.update_joint(self.ip_board_range.get(key).unwrap());
            }
        }
        
        for (key,value) in self.ip_board_range.iter_mut() {
            let community_cards = Hand::from_bit_mask(key.0).count();
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
            let community_cards = Hand::from_bit_mask(key.0).count();
            
            if community_cards < 4 {
                continue;
            }
            
            if self.initial_board.len() == 10 {
                for combo in value.hands.iter_mut() {
                    combo.update_rank(key.0);
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
                let flop_range = &oop_hashmap.get(&(get_card_mask(&self.initial_board), None)).unwrap().hands;
                let mut j = 0;
                let mut reach_probs = vec![0; value.hands.len()];
                for (count, combo) in value.hands.iter().enumerate() {
                    while *combo != flop_range[j] {
                        j += 1;
                    }
                    reach_probs[count] = j as u16;
                    
                }
                self.oop_reach_mapping.insert(*key, reach_probs);
                
                continue;
            } 
            
            // Rivers when solved from flop or turn 
            let turn_range = &oop_hashmap.get(&(key.0, key.1)).unwrap().hands;

            
            let mut j = 0;
            let mut reach_probs = vec![0; value.hands.len()];
            for (count, combo) in value.hands.iter().enumerate() {
                while *combo != turn_range[j] {
                    j += 1;
                }
                reach_probs[count] = j as u16;
                
            }
 
            
            for combo in value.hands.iter_mut() {
                combo.update_rank(key.0);
            }
            
            let hand_range = value.clone();
            let permutation = permutation::sort_by_key(&hand_range.hands, |k| k.3);
            let hand_range = permutation.apply_slice(&hand_range.hands);
            value.hands = hand_range;
            let new_reach_probs = permutation.apply_slice(reach_probs);
            
            self.oop_reach_mapping.insert(*key, new_reach_probs);
        }
        
        for (key,value) in self.ip_board_range.iter_mut() {
            let community_cards = Hand::from_bit_mask(key.0).count();
            
            if community_cards < 4 {
                continue;
            }
            
            if self.initial_board.len() == 10 {
                for combo in value.hands.iter_mut() {
                    combo.update_rank(key.0);
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
                let flop_range = &ip_hashmap.get(&(get_card_mask(&self.initial_board), None)).unwrap().hands;
                let mut j = 0;
                let mut reach_probs = vec![0; value.hands.len()];
                for (count, combo) in value.hands.iter().enumerate() {
                    while *combo != flop_range[j] {
                        j += 1;
                    }
                    reach_probs[count] = j as u16;
                    
                }
                self.ip_reach_mapping.insert(*key, reach_probs);
                
                continue;
            } 
            
            // Rivers when solved from flop or turn 
            let turn_range = &ip_hashmap.get(&(key.0, key.1)).unwrap().hands;
            
            let mut j = 0;
            let mut reach_probs = vec![0; value.hands.len()];
            for (count, combo) in value.hands.iter().enumerate() {
                while *combo != turn_range[j] {
                    j += 1;
                }
                reach_probs[count] = j as u16;
                
            }
            
            for combo in value.hands.iter_mut() {
                combo.update_rank(key.0);
            }
            
            let hand_range = value.clone();
            let permutation = permutation::sort_by_key(&hand_range.hands, |k| k.3);
            let hand_range = permutation.apply_slice(&hand_range.hands);
            value.hands = hand_range;
            let new_reach_probs = permutation.apply_slice(reach_probs);
            
            self.ip_reach_mapping.insert(*key, new_reach_probs);
        }
    }
    
    pub fn get_range(&self, oop: bool, board: u64, previous_board: Option<u64>) -> &HandRange {
        match oop {
            true => self.oop_board_range.get(&(board, previous_board)).unwrap(),
            false => self.ip_board_range.get(&(board, previous_board)).unwrap(),
        }
    }
    
    pub fn get_num_hands(&self, oop: bool, board: u64, previous_board: Option<u64>) -> usize {
        match oop {
            true => self.oop_board_range.get(&(board, previous_board)).unwrap().hands.len(),
            false => self.ip_board_range.get(&(board, previous_board)).unwrap().hands.len(),
        }
    }
    
    pub fn get_initial_reach_probs(&self, oop: bool) -> Vec<f64> {
        let board_mask = get_card_mask(&self.initial_board);
        
        match oop {
            true => {
                let mut reach_probs = vec![0.0; self.oop_board_range.get(&(board_mask, None)).unwrap().hands.len()];
                
                for (i, reach_prob) in reach_probs.iter_mut().enumerate() {
                    *reach_prob = (self.oop_board_range.get(&(board_mask, None)).unwrap().hands[i].2) as f64 / 100.0
                }
                
                reach_probs
            },
            false => {
                let mut reach_probs = vec![0.0; self.ip_board_range.get(&(board_mask, None)).unwrap().hands.len()];
                
                for (i, reach_prob) in reach_probs.iter_mut().enumerate() {
                    *reach_prob = (self.ip_board_range.get(&(board_mask, None)).unwrap().hands[i].2) as f64 / 100.0
                }
                
                reach_probs
            }
        }
    }
    
}