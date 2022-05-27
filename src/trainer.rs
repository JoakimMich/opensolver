use crate::postfloptree::*;
use crate::range::*;
use crate::cfr::*;
use crate::best_response::*;
use std::time::Instant;

pub struct Trainer {
    range_manager: RangeManager,
    root: Node,
}

impl Trainer {
    pub fn new(mut range_manager: RangeManager, mut root: Node, sizings: &SizingSchemes) -> Self {
        range_manager.initialize_ranges();
        recursive_build(None, sizings, &mut root, &range_manager, &range_manager.initial_board);
        Trainer { range_manager, root }
    }
    
    
    
    pub fn train(&mut self, exploitability_goal: f64) {
        let n_iterations = 5000;
        let mut best_response = BestResponse::new(&self.range_manager);
        best_response.set_relative_probablities(true);
        best_response.set_relative_probablities(false);
        let now = Instant::now();
        for i in 0..n_iterations {
            cfr_aux(true, &mut self.root, i, &self.range_manager);
            cfr_aux(false, &mut self.root, i, &self.range_manager);
            if i % 25 == 0 {
                println!("Iteration {}",i);
                let exploitability = best_response.print_exploitability(&self.root);
                if exploitability <= exploitability_goal {
                    break;
                }
            }
        }
        
        println!("Elapsed: {} seconds", now.elapsed().as_secs_f64());
        
        //match self.root.children[0].node_type {
        //    NodeType::ActionNode(ref mut node_info) => {
        //        //println!("{:?} {:?}",node_info.get_average_strategy(), node_info.actions);
        //    },
        //    _ => println!("hmm.."),
        //};
        
        //match self.root.children[0].children[1].node_type {
        //    NodeType::ActionNode(ref mut node_info) => {
        //        //println!("{:?} {:?}",node_info.get_average_strategy(), node_info.actions);
        //    },
        //    _ => println!("hmm.."),
        //};
    }
}

fn cfr_aux(pos: bool, root: &mut Node, n_iteration: u64, range_manager: &RangeManager) {
    let villain_pos = pos ^ true;
    let villain_reach_probs = range_manager.get_initial_reach_probs(villain_pos);
    
    let mut results = vec![];
    let mut cfr_start = CfrState::new(range_manager, &mut results, root, pos, &villain_reach_probs, &range_manager.initial_board, n_iteration);
    cfr_start.run();
}