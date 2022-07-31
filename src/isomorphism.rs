use std::collections::HashMap;

enum FlopType {
    Monotone,
    TwoTone,
    Rainbow,
}

fn flop_type(board: &String) -> FlopType {
    let flop = &board[0..6];
    let iso_mapping = isomorphism_mapping(&flop.to_string());
    let mut counter = 0;
    
    for (from_suit, to_suit) in &iso_mapping {
        if from_suit == to_suit {
            counter += 1;
        }
    }
    
    match counter {
        2 => FlopType::Monotone,
        3 => FlopType::TwoTone,
        4 => FlopType::Rainbow,
        _ => panic!("Invalid suit counter"),
    }
}

pub fn normalize_flop(board: &String) -> String {
    let iso_flag = 0;
    if iso_flag == 0 {
        return board.to_string();
    }
    let mut suits_count: HashMap<char,i64> = HashMap::new();
    
    for i in (0..board.len()).step_by(2) {
        let card = &board[i..i+2].to_lowercase().to_string();
        let suit = card.chars().nth(1).unwrap();
        *suits_count.entry(suit).or_insert(0) += 1;
    }
    
    let key_with_max_value = suits_count.iter().max_by_key(|entry | entry.1).unwrap();
    let key_with_min_value = suits_count.iter().min_by_key(|entry | entry.1).unwrap();
    
    if key_with_max_value.1 == &3 {
        return board.replace(*key_with_max_value.0, "h");
    }
    
    if key_with_max_value.1 == &2 {
        let new_board = board.replace(*key_with_max_value.0, "C").replace(*key_with_min_value.0, "S");
        return new_board.replace("C", "c").replace("S", "s");
    }
    
    
    board.to_string()
}

pub fn isomorphism_mapping(board: &String) -> HashMap<char, char>{
    let iso_flag = 0;
    if iso_flag == 0 {
        let mut iso_mapping = HashMap::new();
        let suits = vec!['h','d','s','c'];
        for s in &suits {
            iso_mapping.insert(*s, *s);
        }
        
        return iso_mapping
    }
    let mut suits_count: HashMap<char,i64> = HashMap::new();
    
    for i in (0..board.len()).step_by(2) {
        let card = &board[i..i+2].to_lowercase().to_string();
        let suit = card.chars().nth(1).unwrap();
        *suits_count.entry(suit).or_insert(0) += 1;
    }
    
    let mut iso_mapping = HashMap::new();
    let suits = vec!['h','d','s','c'];
        
    // Flop
    if board.len() == 6 {
        let key_with_max_value = suits_count.iter().max_by_key(|entry | entry.1).unwrap();
        if key_with_max_value.1 == &3 {
            for s in &suits {
                if s == &'h' {
                    iso_mapping.insert(*s, *s);
                } else {
                    iso_mapping.insert(*s, 'd');
                }
            }
        }
        
        if key_with_max_value.1 == &2 {
            let key_with_min_value = suits_count.iter().min_by_key(|entry | entry.1).unwrap();
            
            for s in &suits {
                if s == key_with_max_value.0 || s == key_with_min_value.0 {
                    iso_mapping.insert(*s, *s);
                } else {
                    iso_mapping.insert(*s, 'h');
                }
            }
            
        }
        
        if key_with_max_value.1 == &1 {
            for s in &suits {
                iso_mapping.insert(*s, *s);
            }
        }
        
    } else if board.len() == 8 {
        let flop_type = flop_type(&board);
        let key_with_max_value = suits_count.iter().max_by_key(|entry | entry.1).unwrap();
        let turn_suit = &board.chars().last().unwrap();
        
        match flop_type {
            FlopType::Monotone => {
                for s in &suits {
                    if s == &'h' || s == &'d' {
                        iso_mapping.insert(*s, *s);
                    } else {
                        if turn_suit == &'h' {
                            iso_mapping.insert(*s, 'd');
                        } else {
                            if s == &'s' {
                                iso_mapping.insert(*s, 'D');
                            } else {
                                iso_mapping.insert(*s, 's');
                            }
                        }
                    }
                }
            },
            FlopType::TwoTone => {
                for s in &suits {
                    if s == &'c' || s == &'s' || s == &'h' {
                        iso_mapping.insert(*s, *s);
                    } else {
                        if turn_suit == &'h' {
                            iso_mapping.insert(*s, 'H');
                        } else {
                            iso_mapping.insert(*s, 'h');
                        }
                    }
                }
            },
            FlopType::Rainbow => {
                for s in &suits {
                    iso_mapping.insert(*s, *s);
                }
            },
        };
        
    } else if board.len() == 10 {
        let flop_type = flop_type(&board);
        let turn_suit = &board.chars().rev().nth(2).unwrap();
        let river_suit = &board.chars().rev().nth(0).unwrap();
        
        match flop_type {
            FlopType::Monotone => {
                if turn_suit == &'h' {
                    for s in &suits {
                        if river_suit == &'h' {
                            if s == &'h' {
                                iso_mapping.insert(*s, *s);
                            } else {
                                iso_mapping.insert(*s, 'd');
                            }
                        } else {
                            if s == &'h' || s == &'d' {
                                iso_mapping.insert(*s, *s);
                            } else {
                                if s == &'s' {
                                    iso_mapping.insert(*s, 'D');
                                } else {
                                    iso_mapping.insert(*s, 's');
                                }
                            }
                            
                        }
                    }
                } else {
                    for s in &suits {
                        if river_suit == &'h' || river_suit == &'d' {
                            if s == &'h' || s == &'d' || s == &'s' {
                                iso_mapping.insert(*s, *s);
                            } else {
                                iso_mapping.insert(*s, 's');
                            }
                        } else if river_suit == &'s' {
                            if s == &'h' || s == &'d' || s == &'s' {
                                iso_mapping.insert(*s, *s);
                            } else {
                                iso_mapping.insert(*s, 'S');
                            }
                        } else {
                            panic!("Impossible river suit (board must be normalized)");
                        }
                    }
                }
            },
            FlopType::TwoTone => {
                if turn_suit == &'c' || turn_suit == &'s' {
                    for s in &suits {
                        if river_suit == &'c' || river_suit == &'s' {
                            if s == &'c' || s == &'s' {
                                iso_mapping.insert(*s, *s);
                            } else {
                                iso_mapping.insert(*s, 'h');
                            }
                        } else {
                            if s == &'c' || s == &'s' || s == &'h' {
                                iso_mapping.insert(*s, *s);
                            } else {
                                iso_mapping.insert(*s, 'H');
                            }
                        }
                    }
                } else if turn_suit == &'h' {
                    for s in &suits {
                        iso_mapping.insert(*s, *s);
                    }
                } else {
                    panic!("Impossible turn suit (board must be normalized)");
                }
            },
            FlopType::Rainbow => {
            },
        };
        
        //for (key, value) in &iso_mapping {
        //    println!("{}: {}", key, value);
        //}
    } else {
        panic!("Unknown board length");
    }
    
    
    iso_mapping
}