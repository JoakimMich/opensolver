use crate::postfloptree::*;
use crate::range::*;
use crate::cfr::*;
use crate::best_response::*;
use std::time::Instant;
use rust_poker::hand_range::{get_card_mask};
use std::collections::HashMap;

pub struct Trainer {
    pub range_manager: RangeManager,
    pub root: Node,
}

pub enum Accuracy {
    Chips(f64),
    Fraction(f64),
}

pub enum TrainFinish {
    Seconds(u64),
    Iterations(u64),
    Indefinite,
}

impl Trainer {
    pub fn new(mut range_manager: RangeManager, lines: Vec<Vec<u32>>, eff_stack: u32, pot_size: u32) -> Self {
        let sizing_mapping = get_sizings(lines);
        range_manager.initialize_ranges();
        let oop_num_hands = range_manager.get_num_hands(true, get_card_mask(&range_manager.initial_board), None);
        let ip_num_hands = range_manager.get_num_hands(false, get_card_mask(&range_manager.initial_board), None);
        let mut root = Node::new_root(eff_stack, pot_size, oop_num_hands, ip_num_hands);
        
        recursive_build(None, &sizing_mapping, &"".to_string(), &mut root, &range_manager, &range_manager.initial_board);
        
        Trainer { range_manager, root }
    }
    
    
    
    pub fn train(&mut self, accuracy: &Accuracy, train_finish: TrainFinish) {
        let mut best_response = BestResponse::new(&self.range_manager);
        best_response.set_relative_probablities(true);
        best_response.set_relative_probablities(false);
        let now = Instant::now();
        let mut i = 0;
        let exploitability_goal = match accuracy {
            Accuracy::Chips(val) => *val,
            Accuracy::Fraction(val) => {
                val * (self.root.pot_size as f64) / 100.0
            },
        };
        let mut time_elapsed = now.elapsed().as_secs_f64();
        loop {
            time_elapsed = now.elapsed().as_secs_f64();
            match train_finish {
                TrainFinish::Seconds(val) => {
                    if time_elapsed as u64 >= val {
                        best_response.print_exploitability(&self.root, time_elapsed);
                        break;
                    }
                },
                TrainFinish::Iterations(val) => {
                    if i >= val {
                        best_response.print_exploitability(&self.root, time_elapsed);
                        break;
                    }
                },
                TrainFinish::Indefinite => (),
            };
            
            cfr_aux(true, &mut self.root, i, &self.range_manager);
            cfr_aux(false, &mut self.root, i, &self.range_manager);
            if i % 25 == 0 {
                let exploitability = best_response.print_exploitability(&self.root, time_elapsed);
                if exploitability <= exploitability_goal {
                    break;
                }
            }
            i += 1;
        }

        //println!("Elapsed: {} seconds", time_elapsed);
    }
}

fn cfr_aux(pos: bool, root: &mut Node, n_iteration: u64, range_manager: &RangeManager) {
    let villain_pos = pos ^ true;
    let villain_reach_probs = range_manager.get_initial_reach_probs(villain_pos);
    let board_mask = get_card_mask(&range_manager.initial_board);
    
    let mut results = vec![];
    let mut cfr_start = CfrState::new(range_manager, &mut results, root, pos, &villain_reach_probs, (board_mask, None), n_iteration);
    cfr_start.run();
}