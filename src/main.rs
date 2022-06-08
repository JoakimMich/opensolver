use crate::postfloptree::*;
use crate::range::*;
use crate::hand_range::*;
use crate::trainer::*;

//TEMP
use crate::isomorphism::*;


mod postfloptree;
mod range;
mod cfr;
mod hand_range;
mod best_response;
mod trainer;
mod isomorphism;

use rust_poker::hand_range::{get_card_mask};


#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    test_flop();
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
    let tree_board = "QsJh2h".to_string();
    let mut rm = RangeManager::new(oop_range, ip_range, tree_board);
    let mut trainer = Trainer::new(rm, &sizing_schemes, eff_stack, pot_size);
    trainer.train(0.5);
}
