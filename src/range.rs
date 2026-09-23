use crate::cards::get_card_mask;
use std::hash::{BuildHasherDefault, Hasher};

/// FxHash: the board keys are trusted u64s, so a multiply-rotate hash is plenty and much faster
/// than the default SipHash
#[derive(Default)]
pub struct FastHasher(u64);

impl Hasher for FastHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_u64(byte as u64);
        }
    }
    #[inline]
    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x517cc1b727220a95);
    }
    #[inline]
    fn write_isize(&mut self, x: isize) {
        self.write_u64(x as u64);
    }
    #[inline]
    fn write_usize(&mut self, x: usize) {
        self.write_u64(x as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

pub type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<FastHasher>>;

use crate::hand_range::*;
use crate::isomorphism::{BoardIsomorphism, board_isomorphism};


extern crate permutation;

#[derive(Debug)]
pub struct RangeManager {
    pub oop_board_range: HashMap<(u64, Option<u64>), HandRange>,
    pub ip_board_range: HashMap<(u64, Option<u64>), HandRange>,
    pub initial_board: String,
    /// Cards dealt at each chance node board, and the isomorphic cards that are skipped
    isomorphisms: HashMap<u64, BoardIsomorphism>,
    /// Skip isomorphic turn and river cards
    pub isomorphism: bool,
    oop_reach_mapping: HashMap<(u64, Option<u64>), Vec<u16>>,
    ip_reach_mapping: HashMap<(u64, Option<u64>), Vec<u16>>,
}

impl RangeManager {
    pub fn initialize_ranges(&mut self) {
        let board_mask = get_card_mask(&self.initial_board);

        self.oop_board_range.get_mut(&(board_mask, None)).unwrap().remove_conflicting_combos(board_mask);
        self.ip_board_range.get_mut(&(board_mask, None)).unwrap().remove_conflicting_combos(board_mask);

        match self.initial_board.len() {
            6 => {
                for turn_card in self.deal(board_mask, &[], self.isomorphism) {
                    let turn_mask = board_mask | (1u64 << turn_card);
                    self.add_ranges(turn_mask, (board_mask, None), (turn_mask, None));
                    for river_card in self.deal(turn_mask, &[board_mask], self.isomorphism) {
                        let river_mask = turn_mask | (1u64 << river_card);
                        self.add_ranges(river_mask, (turn_mask, None), (river_mask, Some(turn_mask)));
                    }
                }
            },
            8 => {
                for river_card in self.deal(board_mask, &[], self.isomorphism) {
                    let river_mask = board_mask | (1u64 << river_card);
                    self.add_ranges(river_mask, (board_mask, None), (river_mask, None));
                }
            },
            10 => (),
            _ => panic!("Initial board invalid length"),
        }

        self.update_ranks();
        self.update_joints();
    }

    /// Decides which cards are dealt to `board_mask` (after `earlier_boards`) and returns them
    fn deal(&mut self, board_mask: u64, earlier_boards: &[u64], isomorphism: bool) -> Vec<u8> {
        let initial_key = (get_card_mask(&self.initial_board), None);
        let starting_ranges = (&self.oop_board_range[&initial_key], &self.ip_board_range[&initial_key]);
        let iso = board_isomorphism(board_mask, earlier_boards, starting_ranges, &self.oop_board_range[&(board_mask, None)], &self.ip_board_range[&(board_mask, None)], isomorphism);
        let deck = iso.deck.clone();
        self.isomorphisms.insert(board_mask, iso);
        deck
    }

    /// Ranges on `board_mask`: the ranges under `from` without the combos the board blocks
    fn add_ranges(&mut self, board_mask: u64, from: (u64, Option<u64>), key: (u64, Option<u64>)) {
        let mut oop_range = self.oop_board_range[&from].clone();
        let mut ip_range = self.ip_board_range[&from].clone();
        oop_range.remove_conflicting_combos(board_mask);
        ip_range.remove_conflicting_combos(board_mask);
        self.oop_board_range.insert(key, oop_range);
        self.ip_board_range.insert(key, ip_range);
    }

    pub fn new(oop_starting_hands: HandRange, ip_starting_hands: HandRange, initial_board: String) -> RangeManager {
        let board_mask = get_card_mask(&initial_board);
        let mut oop_board_range = HashMap::default();
        let mut ip_board_range = HashMap::default();
        oop_board_range.insert((board_mask, None), oop_starting_hands);
        ip_board_range.insert((board_mask, None), ip_starting_hands);

        RangeManager {
            oop_board_range,
            ip_board_range,
            initial_board,
            isomorphisms: HashMap::default(),
            isomorphism: true,
            oop_reach_mapping: HashMap::default(),
            ip_reach_mapping: HashMap::default(),
        }
    }

    /// Cards dealt to a chance node board (isomorphic cards are skipped)
    pub fn get_board_deck(&self, board: u64) -> &Vec<u8> {
        &self.isomorphisms[&board].deck
    }

    /// Isomorphism of a chance node board
    pub fn get_isomorphism(&self, board: u64) -> Option<&BoardIsomorphism> {
        self.isomorphisms.get(&board)
    }
    
    pub fn get_reach_mapping(&self, oop: bool, board: u64, previous_board: Option<u64>) -> &Vec<u16> {
        match oop {
            true => self.oop_reach_mapping.get(&(board, previous_board)).unwrap(),
            false => self.ip_reach_mapping.get(&(board, previous_board)).unwrap(),
        }
    }
    
    
    pub fn update_joints(&mut self) {
        for (key,value) in self.oop_board_range.iter_mut() {
            let community_cards = key.0.count_ones();
            if community_cards != 5 {
                continue;
            }
           
            for combo in value.hands.iter_mut() {
                combo.update_joint(self.ip_board_range.get(key).unwrap());
            }
        }
        
        for (key,value) in self.ip_board_range.iter_mut() {
            let community_cards = key.0.count_ones();
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
            let community_cards = key.0.count_ones();
            
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
            let turn_range = if self.initial_board.len() == 8 {
				&oop_hashmap.get(&(get_card_mask(&self.initial_board), None)).unwrap().hands
			} else {
				&oop_hashmap.get(&(key.1.unwrap(), None)).unwrap().hands
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
            let community_cards = key.0.count_ones();
            
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
            let turn_range = if self.initial_board.len() == 8 {
				&ip_hashmap.get(&(get_card_mask(&self.initial_board), None)).unwrap().hands
			} else {
				&ip_hashmap.get(&(key.1.unwrap(), None)).unwrap().hands
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