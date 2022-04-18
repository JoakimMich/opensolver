use crate::postfloptree::*;
use crate::range::*;
use crate::hand_range::*;
use crate::trainer::*;

mod postfloptree;
mod range;
mod cfr;
mod hand_range;
mod best_response;
mod trainer;

use rust_poker::hand_range::{get_card_mask};


#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    test_flop();
}

fn test_turn() {
    let oop_bet_sizings = vec![0, 100];
    let ip_bet_sizings = vec![0, 100];
    let oop_raise_sizings = vec![];
    let ip_raise_sizings = vec![];
    
    let sizings = SizingSchemes { oop_bet_sizings, ip_bet_sizings, oop_raise_sizings, ip_raise_sizings};
    let eff_stack = 1000;
    let pot_size = 100;
    let mut root = Node::new_root(eff_stack, pot_size);
    let oop_range = HandRange::from_string("QQ+".to_string());
    let ip_range = HandRange::from_string("QQ+".to_string());
    let tree_board = "QsJh2h";
    let mut range_manager = RangeManager::new(oop_range, ip_range, tree_board);
    range_manager.initialize_ranges();
    recursive_build(None, &sizings, &mut root, &range_manager, range_manager.initial_board);
    
    //println!("{:?}",root);
}

fn test_flop() {
    let oop_bet_sizings = vec![0, 100];
    let ip_bet_sizings = vec![0, 100];
    let oop_raise_sizings = vec![100];
    let ip_raise_sizings = vec![100];
    
    let sizing_schemes = SizingSchemes { oop_bet_sizings, ip_bet_sizings, oop_raise_sizings, ip_raise_sizings};
    let eff_stack = 1000;
    let pot_size = 100;
    let oop_range = HandRange::from_string("QQ+".to_string());
    let ip_range = HandRange::from_string("QQ+".to_string());
    let tree_board = "QsJh2h";
    let mut root = Node::new_root(eff_stack, pot_size);
    let mut rm = RangeManager::new(oop_range, ip_range, tree_board);
    let mut trainer = Trainer::new(rm, root, &sizing_schemes);
    trainer.train(0.5);
}

fn test_river() {
    let oop_bet_sizings = vec![0, 100];
    let ip_bet_sizings = vec![0, 100];
    let oop_raise_sizings = vec![100];
    let ip_raise_sizings = vec![100];
    
    let sizing_schemes = SizingSchemes { oop_bet_sizings, ip_bet_sizings, oop_raise_sizings, ip_raise_sizings};
    let eff_stack = 1000;
    let pot_size = 100;
    let root = Node::new_root(eff_stack, pot_size);
    let oop_range = HandRange::from_string("AA,KK,QQ,JJ,TT,99,88,77,66,55,44,AK,AQ,AJ,AT,A9s,A8s,A7s,A6s,A5s,A4s,A3s,A2s,KQ,KJ,KTs,K9s,QJs,QTs,Q9s,JTs,T9s,98s".to_string());
    let ip_range = HandRange::from_string("AA,KK,QQ,JJ,TT,99,88,77,66,55,44,33,22,AK,AQ,AJ,AT,A9,A8,A7,A6,A5,A4,A3,A2s,KQ,KJ,KT,K9,K8s,K7s,K6s,K5s,K4s,K3s,K2s,QJ,QT,Q9,Q8s,Q7s,Q6s,Q5s,Q4s,Q3s,JT,J9,J8s,J7s,J6s,J5s,T9,T8s,T7s,T6s,98,97s,96s,95s,87s,86s,85s,76s,75s,74s,65s,64s,54s,53s".to_string());
    let tree_board = "QsJh2h8d";
    let rm = RangeManager::new(oop_range, ip_range, tree_board);
    let mut trainer = Trainer::new(rm, root, &sizing_schemes);
    trainer.train(0.05);

}