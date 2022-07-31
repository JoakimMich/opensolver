use std::io;
use std::io::Write;
use std::collections::HashMap;

use crate::hand_range::*;
use crate::trainer::*;
use crate::range::*;
#[derive(Debug)]
struct TreeInformation {
    eff_stack: Option<u32>,
    pot: Option<u32>,
    oop_range: Option<HandRange>,
    ip_range: Option<HandRange>,
    lines: Option<Vec<Vec<u32>>>,
    board: Option<String>,
}

pub struct CliSession {
    tree_information: TreeInformation,
    end_string: String,
    accuracy: Accuracy,
    hand_order: Vec<String>,
    hand_order_map: HashMap<String, usize>,
    trainer: Option<Trainer>,
}

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

impl CliSession {
    pub fn new() -> Self {
        let mut tree_information = TreeInformation { eff_stack: None, pot: None, oop_range: None, ip_range: None, lines: None, board: None };
        let mut hand_order = vec!["2d2c".to_string(), "2h2c".to_string(), "2h2d".to_string(), "2s2c".to_string(), "2s2d".to_string(), "2s2h".to_string(), "3c2c".to_string(), "3c2d".to_string(), "3c2h".to_string(), "3c2s".to_string(), "3d2c".to_string(), "3d2d".to_string(), "3d2h".to_string(), "3d2s".to_string(), "3d3c".to_string(), "3h2c".to_string(), "3h2d".to_string(), "3h2h".to_string(), "3h2s".to_string(), "3h3c".to_string(), "3h3d".to_string(), "3s2c".to_string(), "3s2d".to_string(), "3s2h".to_string(), "3s2s".to_string(), "3s3c".to_string(), "3s3d".to_string(), "3s3h".to_string(), "4c2c".to_string(), "4c2d".to_string(), "4c2h".to_string(), "4c2s".to_string(), "4c3c".to_string(), "4c3d".to_string(), "4c3h".to_string(), "4c3s".to_string(), "4d2c".to_string(), "4d2d".to_string(), "4d2h".to_string(), "4d2s".to_string(), "4d3c".to_string(), "4d3d".to_string(), "4d3h".to_string(), "4d3s".to_string(), "4d4c".to_string(), "4h2c".to_string(), "4h2d".to_string(), "4h2h".to_string(), "4h2s".to_string(), "4h3c".to_string(), "4h3d".to_string(), "4h3h".to_string(), "4h3s".to_string(), "4h4c".to_string(), "4h4d".to_string(), "4s2c".to_string(), "4s2d".to_string(), "4s2h".to_string(), "4s2s".to_string(), "4s3c".to_string(), "4s3d".to_string(), "4s3h".to_string(), "4s3s".to_string(), "4s4c".to_string(), "4s4d".to_string(), "4s4h".to_string(), "5c2c".to_string(), "5c2d".to_string(), "5c2h".to_string(), "5c2s".to_string(), "5c3c".to_string(), "5c3d".to_string(), "5c3h".to_string(), "5c3s".to_string(), "5c4c".to_string(), "5c4d".to_string(), "5c4h".to_string(), "5c4s".to_string(), "5d2c".to_string(), "5d2d".to_string(), "5d2h".to_string(), "5d2s".to_string(), "5d3c".to_string(), "5d3d".to_string(), "5d3h".to_string(), "5d3s".to_string(), "5d4c".to_string(), "5d4d".to_string(), "5d4h".to_string(), "5d4s".to_string(), "5d5c".to_string(), "5h2c".to_string(), "5h2d".to_string(), "5h2h".to_string(), "5h2s".to_string(), "5h3c".to_string(), "5h3d".to_string(), "5h3h".to_string(), "5h3s".to_string(), "5h4c".to_string(), "5h4d".to_string(), "5h4h".to_string(), "5h4s".to_string(), "5h5c".to_string(), "5h5d".to_string(), "5s2c".to_string(), "5s2d".to_string(), "5s2h".to_string(), "5s2s".to_string(), "5s3c".to_string(), "5s3d".to_string(), "5s3h".to_string(), "5s3s".to_string(), "5s4c".to_string(), "5s4d".to_string(), "5s4h".to_string(), "5s4s".to_string(), "5s5c".to_string(), "5s5d".to_string(), "5s5h".to_string(), "6c2c".to_string(), "6c2d".to_string(), "6c2h".to_string(), "6c2s".to_string(), "6c3c".to_string(), "6c3d".to_string(), "6c3h".to_string(), "6c3s".to_string(), "6c4c".to_string(), "6c4d".to_string(), "6c4h".to_string(), "6c4s".to_string(), "6c5c".to_string(), "6c5d".to_string(), "6c5h".to_string(), "6c5s".to_string(), "6d2c".to_string(), "6d2d".to_string(), "6d2h".to_string(), "6d2s".to_string(), "6d3c".to_string(), "6d3d".to_string(), "6d3h".to_string(), "6d3s".to_string(), "6d4c".to_string(), "6d4d".to_string(), "6d4h".to_string(), "6d4s".to_string(), "6d5c".to_string(), "6d5d".to_string(), "6d5h".to_string(), "6d5s".to_string(), "6d6c".to_string(), "6h2c".to_string(), "6h2d".to_string(), "6h2h".to_string(), "6h2s".to_string(), "6h3c".to_string(), "6h3d".to_string(), "6h3h".to_string(), "6h3s".to_string(), "6h4c".to_string(), "6h4d".to_string(), "6h4h".to_string(), "6h4s".to_string(), "6h5c".to_string(), "6h5d".to_string(), "6h5h".to_string(), "6h5s".to_string(), "6h6c".to_string(), "6h6d".to_string(), "6s2c".to_string(), "6s2d".to_string(), "6s2h".to_string(), "6s2s".to_string(), "6s3c".to_string(), "6s3d".to_string(), "6s3h".to_string(), "6s3s".to_string(), "6s4c".to_string(), "6s4d".to_string(), "6s4h".to_string(), "6s4s".to_string(), "6s5c".to_string(), "6s5d".to_string(), "6s5h".to_string(), "6s5s".to_string(), "6s6c".to_string(), "6s6d".to_string(), "6s6h".to_string(), "7c2c".to_string(), "7c2d".to_string(), "7c2h".to_string(), "7c2s".to_string(), "7c3c".to_string(), "7c3d".to_string(), "7c3h".to_string(), "7c3s".to_string(), "7c4c".to_string(), "7c4d".to_string(), "7c4h".to_string(), "7c4s".to_string(), "7c5c".to_string(), "7c5d".to_string(), "7c5h".to_string(), "7c5s".to_string(), "7c6c".to_string(), "7c6d".to_string(), "7c6h".to_string(), "7c6s".to_string(), "7d2c".to_string(), "7d2d".to_string(), "7d2h".to_string(), "7d2s".to_string(), "7d3c".to_string(), "7d3d".to_string(), "7d3h".to_string(), "7d3s".to_string(), "7d4c".to_string(), "7d4d".to_string(), "7d4h".to_string(), "7d4s".to_string(), "7d5c".to_string(), "7d5d".to_string(), "7d5h".to_string(), "7d5s".to_string(), "7d6c".to_string(), "7d6d".to_string(), "7d6h".to_string(), "7d6s".to_string(), "7d7c".to_string(), "7h2c".to_string(), "7h2d".to_string(), "7h2h".to_string(), "7h2s".to_string(), "7h3c".to_string(), "7h3d".to_string(), "7h3h".to_string(), "7h3s".to_string(), "7h4c".to_string(), "7h4d".to_string(), "7h4h".to_string(), "7h4s".to_string(), "7h5c".to_string(), "7h5d".to_string(), "7h5h".to_string(), "7h5s".to_string(), "7h6c".to_string(), "7h6d".to_string(), "7h6h".to_string(), "7h6s".to_string(), "7h7c".to_string(), "7h7d".to_string(), "7s2c".to_string(), "7s2d".to_string(), "7s2h".to_string(), "7s2s".to_string(), "7s3c".to_string(), "7s3d".to_string(), "7s3h".to_string(), "7s3s".to_string(), "7s4c".to_string(), "7s4d".to_string(), "7s4h".to_string(), "7s4s".to_string(), "7s5c".to_string(), "7s5d".to_string(), "7s5h".to_string(), "7s5s".to_string(), "7s6c".to_string(), "7s6d".to_string(), "7s6h".to_string(), "7s6s".to_string(), "7s7c".to_string(), "7s7d".to_string(), "7s7h".to_string(), "8c2c".to_string(), "8c2d".to_string(), "8c2h".to_string(), "8c2s".to_string(), "8c3c".to_string(), "8c3d".to_string(), "8c3h".to_string(), "8c3s".to_string(), "8c4c".to_string(), "8c4d".to_string(), "8c4h".to_string(), "8c4s".to_string(), "8c5c".to_string(), "8c5d".to_string(), "8c5h".to_string(), "8c5s".to_string(), "8c6c".to_string(), "8c6d".to_string(), "8c6h".to_string(), "8c6s".to_string(), "8c7c".to_string(), "8c7d".to_string(), "8c7h".to_string(), "8c7s".to_string(), "8d2c".to_string(), "8d2d".to_string(), "8d2h".to_string(), "8d2s".to_string(), "8d3c".to_string(), "8d3d".to_string(), "8d3h".to_string(), "8d3s".to_string(), "8d4c".to_string(), "8d4d".to_string(), "8d4h".to_string(), "8d4s".to_string(), "8d5c".to_string(), "8d5d".to_string(), "8d5h".to_string(), "8d5s".to_string(), "8d6c".to_string(), "8d6d".to_string(), "8d6h".to_string(), "8d6s".to_string(), "8d7c".to_string(), "8d7d".to_string(), "8d7h".to_string(), "8d7s".to_string(), "8d8c".to_string(), "8h2c".to_string(), "8h2d".to_string(), "8h2h".to_string(), "8h2s".to_string(), "8h3c".to_string(), "8h3d".to_string(), "8h3h".to_string(), "8h3s".to_string(), "8h4c".to_string(), "8h4d".to_string(), "8h4h".to_string(), "8h4s".to_string(), "8h5c".to_string(), "8h5d".to_string(), "8h5h".to_string(), "8h5s".to_string(), "8h6c".to_string(), "8h6d".to_string(), "8h6h".to_string(), "8h6s".to_string(), "8h7c".to_string(), "8h7d".to_string(), "8h7h".to_string(), "8h7s".to_string(), "8h8c".to_string(), "8h8d".to_string(), "8s2c".to_string(), "8s2d".to_string(), "8s2h".to_string(), "8s2s".to_string(), "8s3c".to_string(), "8s3d".to_string(), "8s3h".to_string(), "8s3s".to_string(), "8s4c".to_string(), "8s4d".to_string(), "8s4h".to_string(), "8s4s".to_string(), "8s5c".to_string(), "8s5d".to_string(), "8s5h".to_string(), "8s5s".to_string(), "8s6c".to_string(), "8s6d".to_string(), "8s6h".to_string(), "8s6s".to_string(), "8s7c".to_string(), "8s7d".to_string(), "8s7h".to_string(), "8s7s".to_string(), "8s8c".to_string(), "8s8d".to_string(), "8s8h".to_string(), "9c2c".to_string(), "9c2d".to_string(), "9c2h".to_string(), "9c2s".to_string(), "9c3c".to_string(), "9c3d".to_string(), "9c3h".to_string(), "9c3s".to_string(), "9c4c".to_string(), "9c4d".to_string(), "9c4h".to_string(), "9c4s".to_string(), "9c5c".to_string(), "9c5d".to_string(), "9c5h".to_string(), "9c5s".to_string(), "9c6c".to_string(), "9c6d".to_string(), "9c6h".to_string(), "9c6s".to_string(), "9c7c".to_string(), "9c7d".to_string(), "9c7h".to_string(), "9c7s".to_string(), "9c8c".to_string(), "9c8d".to_string(), "9c8h".to_string(), "9c8s".to_string(), "9d2c".to_string(), "9d2d".to_string(), "9d2h".to_string(), "9d2s".to_string(), "9d3c".to_string(), "9d3d".to_string(), "9d3h".to_string(), "9d3s".to_string(), "9d4c".to_string(), "9d4d".to_string(), "9d4h".to_string(), "9d4s".to_string(), "9d5c".to_string(), "9d5d".to_string(), "9d5h".to_string(), "9d5s".to_string(), "9d6c".to_string(), "9d6d".to_string(), "9d6h".to_string(), "9d6s".to_string(), "9d7c".to_string(), "9d7d".to_string(), "9d7h".to_string(), "9d7s".to_string(), "9d8c".to_string(), "9d8d".to_string(), "9d8h".to_string(), "9d8s".to_string(), "9d9c".to_string(), "9h2c".to_string(), "9h2d".to_string(), "9h2h".to_string(), "9h2s".to_string(), "9h3c".to_string(), "9h3d".to_string(), "9h3h".to_string(), "9h3s".to_string(), "9h4c".to_string(), "9h4d".to_string(), "9h4h".to_string(), "9h4s".to_string(), "9h5c".to_string(), "9h5d".to_string(), "9h5h".to_string(), "9h5s".to_string(), "9h6c".to_string(), "9h6d".to_string(), "9h6h".to_string(), "9h6s".to_string(), "9h7c".to_string(), "9h7d".to_string(), "9h7h".to_string(), "9h7s".to_string(), "9h8c".to_string(), "9h8d".to_string(), "9h8h".to_string(), "9h8s".to_string(), "9h9c".to_string(), "9h9d".to_string(), "9s2c".to_string(), "9s2d".to_string(), "9s2h".to_string(), "9s2s".to_string(), "9s3c".to_string(), "9s3d".to_string(), "9s3h".to_string(), "9s3s".to_string(), "9s4c".to_string(), "9s4d".to_string(), "9s4h".to_string(), "9s4s".to_string(), "9s5c".to_string(), "9s5d".to_string(), "9s5h".to_string(), "9s5s".to_string(), "9s6c".to_string(), "9s6d".to_string(), "9s6h".to_string(), "9s6s".to_string(), "9s7c".to_string(), "9s7d".to_string(), "9s7h".to_string(), "9s7s".to_string(), "9s8c".to_string(), "9s8d".to_string(), "9s8h".to_string(), "9s8s".to_string(), "9s9c".to_string(), "9s9d".to_string(), "9s9h".to_string(), "Tc2c".to_string(), "Tc2d".to_string(), "Tc2h".to_string(), "Tc2s".to_string(), "Tc3c".to_string(), "Tc3d".to_string(), "Tc3h".to_string(), "Tc3s".to_string(), "Tc4c".to_string(), "Tc4d".to_string(), "Tc4h".to_string(), "Tc4s".to_string(), "Tc5c".to_string(), "Tc5d".to_string(), "Tc5h".to_string(), "Tc5s".to_string(), "Tc6c".to_string(), "Tc6d".to_string(), "Tc6h".to_string(), "Tc6s".to_string(), "Tc7c".to_string(), "Tc7d".to_string(), "Tc7h".to_string(), "Tc7s".to_string(), "Tc8c".to_string(), "Tc8d".to_string(), "Tc8h".to_string(), "Tc8s".to_string(), "Tc9c".to_string(), "Tc9d".to_string(), "Tc9h".to_string(), "Tc9s".to_string(), "Td2c".to_string(), "Td2d".to_string(), "Td2h".to_string(), "Td2s".to_string(), "Td3c".to_string(), "Td3d".to_string(), "Td3h".to_string(), "Td3s".to_string(), "Td4c".to_string(), "Td4d".to_string(), "Td4h".to_string(), "Td4s".to_string(), "Td5c".to_string(), "Td5d".to_string(), "Td5h".to_string(), "Td5s".to_string(), "Td6c".to_string(), "Td6d".to_string(), "Td6h".to_string(), "Td6s".to_string(), "Td7c".to_string(), "Td7d".to_string(), "Td7h".to_string(), "Td7s".to_string(), "Td8c".to_string(), "Td8d".to_string(), "Td8h".to_string(), "Td8s".to_string(), "Td9c".to_string(), "Td9d".to_string(), "Td9h".to_string(), "Td9s".to_string(), "TdTc".to_string(), "Th2c".to_string(), "Th2d".to_string(), "Th2h".to_string(), "Th2s".to_string(), "Th3c".to_string(), "Th3d".to_string(), "Th3h".to_string(), "Th3s".to_string(), "Th4c".to_string(), "Th4d".to_string(), "Th4h".to_string(), "Th4s".to_string(), "Th5c".to_string(), "Th5d".to_string(), "Th5h".to_string(), "Th5s".to_string(), "Th6c".to_string(), "Th6d".to_string(), "Th6h".to_string(), "Th6s".to_string(), "Th7c".to_string(), "Th7d".to_string(), "Th7h".to_string(), "Th7s".to_string(), "Th8c".to_string(), "Th8d".to_string(), "Th8h".to_string(), "Th8s".to_string(), "Th9c".to_string(), "Th9d".to_string(), "Th9h".to_string(), "Th9s".to_string(), "ThTc".to_string(), "ThTd".to_string(), "Ts2c".to_string(), "Ts2d".to_string(), "Ts2h".to_string(), "Ts2s".to_string(), "Ts3c".to_string(), "Ts3d".to_string(), "Ts3h".to_string(), "Ts3s".to_string(), "Ts4c".to_string(), "Ts4d".to_string(), "Ts4h".to_string(), "Ts4s".to_string(), "Ts5c".to_string(), "Ts5d".to_string(), "Ts5h".to_string(), "Ts5s".to_string(), "Ts6c".to_string(), "Ts6d".to_string(), "Ts6h".to_string(), "Ts6s".to_string(), "Ts7c".to_string(), "Ts7d".to_string(), "Ts7h".to_string(), "Ts7s".to_string(), "Ts8c".to_string(), "Ts8d".to_string(), "Ts8h".to_string(), "Ts8s".to_string(), "Ts9c".to_string(), "Ts9d".to_string(), "Ts9h".to_string(), "Ts9s".to_string(), "TsTc".to_string(), "TsTd".to_string(), "TsTh".to_string(), "Jc2c".to_string(), "Jc2d".to_string(), "Jc2h".to_string(), "Jc2s".to_string(), "Jc3c".to_string(), "Jc3d".to_string(), "Jc3h".to_string(), "Jc3s".to_string(), "Jc4c".to_string(), "Jc4d".to_string(), "Jc4h".to_string(), "Jc4s".to_string(), "Jc5c".to_string(), "Jc5d".to_string(), "Jc5h".to_string(), "Jc5s".to_string(), "Jc6c".to_string(), "Jc6d".to_string(), "Jc6h".to_string(), "Jc6s".to_string(), "Jc7c".to_string(), "Jc7d".to_string(), "Jc7h".to_string(), "Jc7s".to_string(), "Jc8c".to_string(), "Jc8d".to_string(), "Jc8h".to_string(), "Jc8s".to_string(), "Jc9c".to_string(), "Jc9d".to_string(), "Jc9h".to_string(), "Jc9s".to_string(), "JcTc".to_string(), "JcTd".to_string(), "JcTh".to_string(), "JcTs".to_string(), "Jd2c".to_string(), "Jd2d".to_string(), "Jd2h".to_string(), "Jd2s".to_string(), "Jd3c".to_string(), "Jd3d".to_string(), "Jd3h".to_string(), "Jd3s".to_string(), "Jd4c".to_string(), "Jd4d".to_string(), "Jd4h".to_string(), "Jd4s".to_string(), "Jd5c".to_string(), "Jd5d".to_string(), "Jd5h".to_string(), "Jd5s".to_string(), "Jd6c".to_string(), "Jd6d".to_string(), "Jd6h".to_string(), "Jd6s".to_string(), "Jd7c".to_string(), "Jd7d".to_string(), "Jd7h".to_string(), "Jd7s".to_string(), "Jd8c".to_string(), "Jd8d".to_string(), "Jd8h".to_string(), "Jd8s".to_string(), "Jd9c".to_string(), "Jd9d".to_string(), "Jd9h".to_string(), "Jd9s".to_string(), "JdTc".to_string(), "JdTd".to_string(), "JdTh".to_string(), "JdTs".to_string(), "JdJc".to_string(), "Jh2c".to_string(), "Jh2d".to_string(), "Jh2h".to_string(), "Jh2s".to_string(), "Jh3c".to_string(), "Jh3d".to_string(), "Jh3h".to_string(), "Jh3s".to_string(), "Jh4c".to_string(), "Jh4d".to_string(), "Jh4h".to_string(), "Jh4s".to_string(), "Jh5c".to_string(), "Jh5d".to_string(), "Jh5h".to_string(), "Jh5s".to_string(), "Jh6c".to_string(), "Jh6d".to_string(), "Jh6h".to_string(), "Jh6s".to_string(), "Jh7c".to_string(), "Jh7d".to_string(), "Jh7h".to_string(), "Jh7s".to_string(), "Jh8c".to_string(), "Jh8d".to_string(), "Jh8h".to_string(), "Jh8s".to_string(), "Jh9c".to_string(), "Jh9d".to_string(), "Jh9h".to_string(), "Jh9s".to_string(), "JhTc".to_string(), "JhTd".to_string(), "JhTh".to_string(), "JhTs".to_string(), "JhJc".to_string(), "JhJd".to_string(), "Js2c".to_string(), "Js2d".to_string(), "Js2h".to_string(), "Js2s".to_string(), "Js3c".to_string(), "Js3d".to_string(), "Js3h".to_string(), "Js3s".to_string(), "Js4c".to_string(), "Js4d".to_string(), "Js4h".to_string(), "Js4s".to_string(), "Js5c".to_string(), "Js5d".to_string(), "Js5h".to_string(), "Js5s".to_string(), "Js6c".to_string(), "Js6d".to_string(), "Js6h".to_string(), "Js6s".to_string(), "Js7c".to_string(), "Js7d".to_string(), "Js7h".to_string(), "Js7s".to_string(), "Js8c".to_string(), "Js8d".to_string(), "Js8h".to_string(), "Js8s".to_string(), "Js9c".to_string(), "Js9d".to_string(), "Js9h".to_string(), "Js9s".to_string(), "JsTc".to_string(), "JsTd".to_string(), "JsTh".to_string(), "JsTs".to_string(), "JsJc".to_string(), "JsJd".to_string(), "JsJh".to_string(), "Qc2c".to_string(), "Qc2d".to_string(), "Qc2h".to_string(), "Qc2s".to_string(), "Qc3c".to_string(), "Qc3d".to_string(), "Qc3h".to_string(), "Qc3s".to_string(), "Qc4c".to_string(), "Qc4d".to_string(), "Qc4h".to_string(), "Qc4s".to_string(), "Qc5c".to_string(), "Qc5d".to_string(), "Qc5h".to_string(), "Qc5s".to_string(), "Qc6c".to_string(), "Qc6d".to_string(), "Qc6h".to_string(), "Qc6s".to_string(), "Qc7c".to_string(), "Qc7d".to_string(), "Qc7h".to_string(), "Qc7s".to_string(), "Qc8c".to_string(), "Qc8d".to_string(), "Qc8h".to_string(), "Qc8s".to_string(), "Qc9c".to_string(), "Qc9d".to_string(), "Qc9h".to_string(), "Qc9s".to_string(), "QcTc".to_string(), "QcTd".to_string(), "QcTh".to_string(), "QcTs".to_string(), "QcJc".to_string(), "QcJd".to_string(), "QcJh".to_string(), "QcJs".to_string(), "Qd2c".to_string(), "Qd2d".to_string(), "Qd2h".to_string(), "Qd2s".to_string(), "Qd3c".to_string(), "Qd3d".to_string(), "Qd3h".to_string(), "Qd3s".to_string(), "Qd4c".to_string(), "Qd4d".to_string(), "Qd4h".to_string(), "Qd4s".to_string(), "Qd5c".to_string(), "Qd5d".to_string(), "Qd5h".to_string(), "Qd5s".to_string(), "Qd6c".to_string(), "Qd6d".to_string(), "Qd6h".to_string(), "Qd6s".to_string(), "Qd7c".to_string(), "Qd7d".to_string(), "Qd7h".to_string(), "Qd7s".to_string(), "Qd8c".to_string(), "Qd8d".to_string(), "Qd8h".to_string(), "Qd8s".to_string(), "Qd9c".to_string(), "Qd9d".to_string(), "Qd9h".to_string(), "Qd9s".to_string(), "QdTc".to_string(), "QdTd".to_string(), "QdTh".to_string(), "QdTs".to_string(), "QdJc".to_string(), "QdJd".to_string(), "QdJh".to_string(), "QdJs".to_string(), "QdQc".to_string(), "Qh2c".to_string(), "Qh2d".to_string(), "Qh2h".to_string(), "Qh2s".to_string(), "Qh3c".to_string(), "Qh3d".to_string(), "Qh3h".to_string(), "Qh3s".to_string(), "Qh4c".to_string(), "Qh4d".to_string(), "Qh4h".to_string(), "Qh4s".to_string(), "Qh5c".to_string(), "Qh5d".to_string(), "Qh5h".to_string(), "Qh5s".to_string(), "Qh6c".to_string(), "Qh6d".to_string(), "Qh6h".to_string(), "Qh6s".to_string(), "Qh7c".to_string(), "Qh7d".to_string(), "Qh7h".to_string(), "Qh7s".to_string(), "Qh8c".to_string(), "Qh8d".to_string(), "Qh8h".to_string(), "Qh8s".to_string(), "Qh9c".to_string(), "Qh9d".to_string(), "Qh9h".to_string(), "Qh9s".to_string(), "QhTc".to_string(), "QhTd".to_string(), "QhTh".to_string(), "QhTs".to_string(), "QhJc".to_string(), "QhJd".to_string(), "QhJh".to_string(), "QhJs".to_string(), "QhQc".to_string(), "QhQd".to_string(), "Qs2c".to_string(), "Qs2d".to_string(), "Qs2h".to_string(), "Qs2s".to_string(), "Qs3c".to_string(), "Qs3d".to_string(), "Qs3h".to_string(), "Qs3s".to_string(), "Qs4c".to_string(), "Qs4d".to_string(), "Qs4h".to_string(), "Qs4s".to_string(), "Qs5c".to_string(), "Qs5d".to_string(), "Qs5h".to_string(), "Qs5s".to_string(), "Qs6c".to_string(), "Qs6d".to_string(), "Qs6h".to_string(), "Qs6s".to_string(), "Qs7c".to_string(), "Qs7d".to_string(), "Qs7h".to_string(), "Qs7s".to_string(), "Qs8c".to_string(), "Qs8d".to_string(), "Qs8h".to_string(), "Qs8s".to_string(), "Qs9c".to_string(), "Qs9d".to_string(), "Qs9h".to_string(), "Qs9s".to_string(), "QsTc".to_string(), "QsTd".to_string(), "QsTh".to_string(), "QsTs".to_string(), "QsJc".to_string(), "QsJd".to_string(), "QsJh".to_string(), "QsJs".to_string(), "QsQc".to_string(), "QsQd".to_string(), "QsQh".to_string(), "Kc2c".to_string(), "Kc2d".to_string(), "Kc2h".to_string(), "Kc2s".to_string(), "Kc3c".to_string(), "Kc3d".to_string(), "Kc3h".to_string(), "Kc3s".to_string(), "Kc4c".to_string(), "Kc4d".to_string(), "Kc4h".to_string(), "Kc4s".to_string(), "Kc5c".to_string(), "Kc5d".to_string(), "Kc5h".to_string(), "Kc5s".to_string(), "Kc6c".to_string(), "Kc6d".to_string(), "Kc6h".to_string(), "Kc6s".to_string(), "Kc7c".to_string(), "Kc7d".to_string(), "Kc7h".to_string(), "Kc7s".to_string(), "Kc8c".to_string(), "Kc8d".to_string(), "Kc8h".to_string(), "Kc8s".to_string(), "Kc9c".to_string(), "Kc9d".to_string(), "Kc9h".to_string(), "Kc9s".to_string(), "KcTc".to_string(), "KcTd".to_string(), "KcTh".to_string(), "KcTs".to_string(), "KcJc".to_string(), "KcJd".to_string(), "KcJh".to_string(), "KcJs".to_string(), "KcQc".to_string(), "KcQd".to_string(), "KcQh".to_string(), "KcQs".to_string(), "Kd2c".to_string(), "Kd2d".to_string(), "Kd2h".to_string(), "Kd2s".to_string(), "Kd3c".to_string(), "Kd3d".to_string(), "Kd3h".to_string(), "Kd3s".to_string(), "Kd4c".to_string(), "Kd4d".to_string(), "Kd4h".to_string(), "Kd4s".to_string(), "Kd5c".to_string(), "Kd5d".to_string(), "Kd5h".to_string(), "Kd5s".to_string(), "Kd6c".to_string(), "Kd6d".to_string(), "Kd6h".to_string(), "Kd6s".to_string(), "Kd7c".to_string(), "Kd7d".to_string(), "Kd7h".to_string(), "Kd7s".to_string(), "Kd8c".to_string(), "Kd8d".to_string(), "Kd8h".to_string(), "Kd8s".to_string(), "Kd9c".to_string(), "Kd9d".to_string(), "Kd9h".to_string(), "Kd9s".to_string(), "KdTc".to_string(), "KdTd".to_string(), "KdTh".to_string(), "KdTs".to_string(), "KdJc".to_string(), "KdJd".to_string(), "KdJh".to_string(), "KdJs".to_string(), "KdQc".to_string(), "KdQd".to_string(), "KdQh".to_string(), "KdQs".to_string(), "KdKc".to_string(), "Kh2c".to_string(), "Kh2d".to_string(), "Kh2h".to_string(), "Kh2s".to_string(), "Kh3c".to_string(), "Kh3d".to_string(), "Kh3h".to_string(), "Kh3s".to_string(), "Kh4c".to_string(), "Kh4d".to_string(), "Kh4h".to_string(), "Kh4s".to_string(), "Kh5c".to_string(), "Kh5d".to_string(), "Kh5h".to_string(), "Kh5s".to_string(), "Kh6c".to_string(), "Kh6d".to_string(), "Kh6h".to_string(), "Kh6s".to_string(), "Kh7c".to_string(), "Kh7d".to_string(), "Kh7h".to_string(), "Kh7s".to_string(), "Kh8c".to_string(), "Kh8d".to_string(), "Kh8h".to_string(), "Kh8s".to_string(), "Kh9c".to_string(), "Kh9d".to_string(), "Kh9h".to_string(), "Kh9s".to_string(), "KhTc".to_string(), "KhTd".to_string(), "KhTh".to_string(), "KhTs".to_string(), "KhJc".to_string(), "KhJd".to_string(), "KhJh".to_string(), "KhJs".to_string(), "KhQc".to_string(), "KhQd".to_string(), "KhQh".to_string(), "KhQs".to_string(), "KhKc".to_string(), "KhKd".to_string(), "Ks2c".to_string(), "Ks2d".to_string(), "Ks2h".to_string(), "Ks2s".to_string(), "Ks3c".to_string(), "Ks3d".to_string(), "Ks3h".to_string(), "Ks3s".to_string(), "Ks4c".to_string(), "Ks4d".to_string(), "Ks4h".to_string(), "Ks4s".to_string(), "Ks5c".to_string(), "Ks5d".to_string(), "Ks5h".to_string(), "Ks5s".to_string(), "Ks6c".to_string(), "Ks6d".to_string(), "Ks6h".to_string(), "Ks6s".to_string(), "Ks7c".to_string(), "Ks7d".to_string(), "Ks7h".to_string(), "Ks7s".to_string(), "Ks8c".to_string(), "Ks8d".to_string(), "Ks8h".to_string(), "Ks8s".to_string(), "Ks9c".to_string(), "Ks9d".to_string(), "Ks9h".to_string(), "Ks9s".to_string(), "KsTc".to_string(), "KsTd".to_string(), "KsTh".to_string(), "KsTs".to_string(), "KsJc".to_string(), "KsJd".to_string(), "KsJh".to_string(), "KsJs".to_string(), "KsQc".to_string(), "KsQd".to_string(), "KsQh".to_string(), "KsQs".to_string(), "KsKc".to_string(), "KsKd".to_string(), "KsKh".to_string(), "Ac2c".to_string(), "Ac2d".to_string(), "Ac2h".to_string(), "Ac2s".to_string(), "Ac3c".to_string(), "Ac3d".to_string(), "Ac3h".to_string(), "Ac3s".to_string(), "Ac4c".to_string(), "Ac4d".to_string(), "Ac4h".to_string(), "Ac4s".to_string(), "Ac5c".to_string(), "Ac5d".to_string(), "Ac5h".to_string(), "Ac5s".to_string(), "Ac6c".to_string(), "Ac6d".to_string(), "Ac6h".to_string(), "Ac6s".to_string(), "Ac7c".to_string(), "Ac7d".to_string(), "Ac7h".to_string(), "Ac7s".to_string(), "Ac8c".to_string(), "Ac8d".to_string(), "Ac8h".to_string(), "Ac8s".to_string(), "Ac9c".to_string(), "Ac9d".to_string(), "Ac9h".to_string(), "Ac9s".to_string(), "AcTc".to_string(), "AcTd".to_string(), "AcTh".to_string(), "AcTs".to_string(), "AcJc".to_string(), "AcJd".to_string(), "AcJh".to_string(), "AcJs".to_string(), "AcQc".to_string(), "AcQd".to_string(), "AcQh".to_string(), "AcQs".to_string(), "AcKc".to_string(), "AcKd".to_string(), "AcKh".to_string(), "AcKs".to_string(), "Ad2c".to_string(), "Ad2d".to_string(), "Ad2h".to_string(), "Ad2s".to_string(), "Ad3c".to_string(), "Ad3d".to_string(), "Ad3h".to_string(), "Ad3s".to_string(), "Ad4c".to_string(), "Ad4d".to_string(), "Ad4h".to_string(), "Ad4s".to_string(), "Ad5c".to_string(), "Ad5d".to_string(), "Ad5h".to_string(), "Ad5s".to_string(), "Ad6c".to_string(), "Ad6d".to_string(), "Ad6h".to_string(), "Ad6s".to_string(), "Ad7c".to_string(), "Ad7d".to_string(), "Ad7h".to_string(), "Ad7s".to_string(), "Ad8c".to_string(), "Ad8d".to_string(), "Ad8h".to_string(), "Ad8s".to_string(), "Ad9c".to_string(), "Ad9d".to_string(), "Ad9h".to_string(), "Ad9s".to_string(), "AdTc".to_string(), "AdTd".to_string(), "AdTh".to_string(), "AdTs".to_string(), "AdJc".to_string(), "AdJd".to_string(), "AdJh".to_string(), "AdJs".to_string(), "AdQc".to_string(), "AdQd".to_string(), "AdQh".to_string(), "AdQs".to_string(), "AdKc".to_string(), "AdKd".to_string(), "AdKh".to_string(), "AdKs".to_string(), "AdAc".to_string(), "Ah2c".to_string(), "Ah2d".to_string(), "Ah2h".to_string(), "Ah2s".to_string(), "Ah3c".to_string(), "Ah3d".to_string(), "Ah3h".to_string(), "Ah3s".to_string(), "Ah4c".to_string(), "Ah4d".to_string(), "Ah4h".to_string(), "Ah4s".to_string(), "Ah5c".to_string(), "Ah5d".to_string(), "Ah5h".to_string(), "Ah5s".to_string(), "Ah6c".to_string(), "Ah6d".to_string(), "Ah6h".to_string(), "Ah6s".to_string(), "Ah7c".to_string(), "Ah7d".to_string(), "Ah7h".to_string(), "Ah7s".to_string(), "Ah8c".to_string(), "Ah8d".to_string(), "Ah8h".to_string(), "Ah8s".to_string(), "Ah9c".to_string(), "Ah9d".to_string(), "Ah9h".to_string(), "Ah9s".to_string(), "AhTc".to_string(), "AhTd".to_string(), "AhTh".to_string(), "AhTs".to_string(), "AhJc".to_string(), "AhJd".to_string(), "AhJh".to_string(), "AhJs".to_string(), "AhQc".to_string(), "AhQd".to_string(), "AhQh".to_string(), "AhQs".to_string(), "AhKc".to_string(), "AhKd".to_string(), "AhKh".to_string(), "AhKs".to_string(), "AhAc".to_string(), "AhAd".to_string(), "As2c".to_string(), "As2d".to_string(), "As2h".to_string(), "As2s".to_string(), "As3c".to_string(), "As3d".to_string(), "As3h".to_string(), "As3s".to_string(), "As4c".to_string(), "As4d".to_string(), "As4h".to_string(), "As4s".to_string(), "As5c".to_string(), "As5d".to_string(), "As5h".to_string(), "As5s".to_string(), "As6c".to_string(), "As6d".to_string(), "As6h".to_string(), "As6s".to_string(), "As7c".to_string(), "As7d".to_string(), "As7h".to_string(), "As7s".to_string(), "As8c".to_string(), "As8d".to_string(), "As8h".to_string(), "As8s".to_string(), "As9c".to_string(), "As9d".to_string(), "As9h".to_string(), "As9s".to_string(), "AsTc".to_string(), "AsTd".to_string(), "AsTh".to_string(), "AsTs".to_string(), "AsJc".to_string(), "AsJd".to_string(), "AsJh".to_string(), "AsJs".to_string(), "AsQc".to_string(), "AsQd".to_string(), "AsQh".to_string(), "AsQs".to_string(), "AsKc".to_string(), "AsKd".to_string(), "AsKh".to_string(), "AsKs".to_string(), "AsAc".to_string(), "AsAd".to_string(), "AsAh".to_string()];
        let mut hand_order_map = HashMap::new();
        
        for (i,hand) in hand_order.iter().enumerate() {
            hand_order_map.insert(hand.clone(), i);
        }
        
        CliSession { tree_information, end_string: "".to_string(), accuracy: Accuracy::Chips(0.0), hand_order, hand_order_map, trainer: None }
    }
    
    pub fn start(&mut self) {
        let mut user_input = String::new();
        println!("OpenSolver free (piosolver) 0.0.1 (Jul 29 2022, 17:48)");
        println!("(C) John Doe");
        while true {
            user_input.clear();
            io::stdout().flush().expect("Cannot flush stdout");
            io::stdin()
                .read_line(&mut user_input)
                .expect("Cannot read user input");
            trim_newline(&mut user_input);
            
            if user_input.len() != 0 && user_input.chars().nth(0).unwrap() != '#' {
                let mut split = user_input.as_str().split(" ");
                let input_params = split.collect::<Vec<&str>>();
                match input_params[0] {
                    "set_end_string" => set_end_string(&input_params, &mut self.end_string),
                    "set_accuracy" => set_accuracy(&input_params, &mut self.accuracy),
                    "set_eff_stack" => set_eff_stack(&input_params, &mut self.tree_information),
                    "set_pot" => set_pot(&input_params, &mut self.tree_information),
                    "set_board" => set_board(&input_params, &mut self.tree_information),
                    "show_effective_stack" => {
                        if let Some(x) = self.tree_information.eff_stack {
                            println!("{}",x);
                        } else {
                            println!("ERROR: {} missing/incorrect tree", input_params[0])
                        }
                    },
                    "show_children" => show_children(&input_params, &self.trainer),
                    "show_range" => show_range(&input_params, &self.trainer, &self.hand_order_map),
                    "show_strategy" => show_strategy(&input_params, &self.trainer, &self.hand_order_map),
                    "calc_line_freq" => calc_line_freq(&input_params, &self.trainer, &self.hand_order_map),
                    "calc_eq_node" => calc_eq_node(&input_params, &self.trainer),
                    "calc_ev" => calc_ev(&input_params, &self.trainer),
                    "show_node" => show_node(&input_params, &self.trainer),
                    "add_line" => add_line(&input_params, &mut self.tree_information),
                    "clear_lines" => clear_lines(&mut self.tree_information),
                    "build_tree" => build_tree(&mut self.tree_information, &mut self.trainer),
                    "is_ready" => println!("{} ok!", input_params[0]),
                    "set_isomorphism" => println!("{} ok!", input_params[0]), // TODO: fix this
                    "set_threads" => println!("{} ok!", input_params[0]), // TODO: fix this
                    "set_recalc_accuracy" => println!("{} ok!", input_params[0]), // TODO: fix this
                    "show_hand_order" => println!("{:?}",self.hand_order),
                    "set_range" => set_range(&input_params, &mut self.tree_information,&self.hand_order),
                    "go" => go(&input_params, &mut self.trainer, &self.accuracy, &self.end_string),
                    "exit" => break,
                    _ => println!("ERROR: Command {} not recognized", input_params[0]),
                };
                if self.end_string.len() > 0 && input_params[0] != "go" {
                    println!("{}",self.end_string);
                }
            }
        }
    }
}

fn set_end_string(input_params: &Vec<&str>, end_string: &mut String) {
    if input_params.len() == 1 {
        println!("ERROR: {} incorrect or missing argument", input_params[0]);
    } else {
        *end_string = input_params[1].to_string();
        println!("{} ok!", input_params[0]);
    }
}

fn set_accuracy(input_params: &Vec<&str>, accuracy: &mut Accuracy) {
    // TODO: add optional argument chips or fraction
    if input_params.len() == 1 {
        println!("ERROR: {} incorrect or missing argument", input_params[0]);
    } else if input_params[1].parse::<f64>().is_ok() == false || input_params[1].parse::<f64>().unwrap() < 0.0 {
        println!("ERROR: Invalid value");
    } else {
        let mut accuracy_type = if input_params.len() > 2 {
            match input_params[2] {
                "fraction" => Accuracy::Fraction(input_params[1].parse::<f64>().unwrap()),
                _ => Accuracy::Chips(input_params[1].parse::<f64>().unwrap())
            }
        } else {
            Accuracy::Chips(input_params[1].parse::<f64>().unwrap())
        };
        *accuracy = accuracy_type;
        println!("{} ok!", input_params[0]);
    }
}

fn set_eff_stack(input_params: &Vec<&str>, tree_information: &mut TreeInformation) {
    if input_params.len() == 1 {
        println!("ERROR: {} incorrect or missing argument", input_params[0]);
    } else if input_params[1].parse::<u32>().is_ok() == false {
        println!("ERROR: Invalid value");
    } else {
        tree_information.eff_stack = Some(input_params[1].parse::<u32>().unwrap());
        println!("{} ok!", input_params[0]);
    }
}

fn set_pot(input_params: &Vec<&str>, tree_information: &mut TreeInformation) {
    if input_params.len() < 4 {
        println!("ERROR: {} incorrect or missing argument", input_params[0]);
    } else if input_params[3].parse::<u32>().is_ok() == false {
        println!("ERROR: Invalid value");
    } else {
        tree_information.pot = Some(input_params[3].parse::<u32>().unwrap());
        println!("{} ok!", input_params[0]);
    }
}

fn set_board(input_params: &Vec<&str>, tree_information: &mut TreeInformation) {
    //TODO: error handling for invalid boards
    if input_params.len() == 1 {
        println!("ERROR: {} incorrect or missing argument", input_params[0]);
    } else {
        tree_information.board = Some(input_params[1].to_string());
        println!("{} ok!", input_params[0]);
    }
}

fn set_range(input_params: &Vec<&str>, tree_information: &mut TreeInformation, hand_order: &Vec<String>) {
    if input_params.len() < 1328 {
        println!("ERROR: {} incorrect or missing argument", input_params[0]);
    } else if (input_params[1] != "OOP" && input_params[1] != "IP") {
        println!("ERROR: {} incorrect player", input_params[0]);
    } else {
        let mut hand_range_string = String::new();
        for i in 0..1326 {
            let hand_weight = if input_params[i+2].parse::<f64>().is_ok() == false || input_params[i+2].parse::<f64>().unwrap() < 0.0 {
                0.0
            } else if input_params[i+2].parse::<f64>().unwrap() > 1.0 {
                1.0
            } else {
                input_params[i+2].parse::<f64>().unwrap()
            };
            
            if hand_weight > 0.0 {
                let new_hand = hand_order[i].clone();
                let new_weight = (hand_weight*100.0) as u8;
                hand_range_string = format!("{}{}@{},",hand_range_string,new_hand,new_weight);
            }
        }
        hand_range_string.pop();
        let range = HandRange::from_string(hand_range_string);
        if input_params[1] == "OOP" {
            tree_information.oop_range = Some(range);
        } else {
            tree_information.ip_range = Some(range);
        }
        println!("{} ok!", input_params[0]);
    }
}

fn show_children(input_params: &Vec<&str>, trainer_option: &Option<Trainer>) {
    match trainer_option {
        Some(trainer) => {
            if input_params.len() == 1 {
                println!("ERROR: {} incorrect or missing argument", input_params[0]);
            } else {
                let children_info = trainer.root.get_children(input_params[1].to_string(), &trainer.range_manager);
                for (i,info) in children_info.iter().enumerate() {
                    println!("child {}:", i);
                    println!("{}",info);
                }
            }
        },
        None => println!("ERROR: Built tree not found"),
    };
}

fn show_range(input_params: &Vec<&str>, trainer_option: &Option<Trainer>, hand_order_map: &HashMap<String, usize>) {
    match trainer_option {
        Some(trainer) => {
            if input_params.len() < 3 {
                println!("ERROR: {} incorrect or missing argument", input_params[0]);
            } else {
                let oop = if input_params[1] == "oop" || input_params[1] == "OOP" {
                    true
                } else if input_params[1] == "ip" || input_params[1] == "IP" {
                    false
                } else {
                    println!("ERROR: {} incorrect or missing argument", input_params[0]);
                    return;
                };
                let range = trainer.root.get_range(oop, input_params[2].to_string(), &trainer.range_manager, hand_order_map);
                for el in &range {
                     print!("{} ", el);
                }
                println!("");
            }
        },
        None => println!("ERROR: Built tree not found"),
    };
}

fn show_strategy(input_params: &Vec<&str>, trainer_option: &Option<Trainer>, hand_order_map: &HashMap<String, usize>) {
    match trainer_option {
        Some(trainer) => {
            if input_params.len() < 2 {
                println!("ERROR: {} incorrect or missing argument", input_params[0]);
            } else {
                let strategy = trainer.root.get_strategy(input_params[1].to_string(), &trainer.range_manager, hand_order_map);
                for action_strategy in &strategy {
                    for el in action_strategy {
                        print!("{} ", el);
                    }
                    println!("");
                }
            }
        },
        None => println!("ERROR: Built tree not found"),
    };
}

fn calc_line_freq(input_params: &Vec<&str>, trainer_option: &Option<Trainer>, hand_order_map: &HashMap<String, usize>) {
    match trainer_option {
        Some(trainer) => {
            if input_params.len() < 2 {
                println!("ERROR: {} incorrect or missing argument", input_params[0]);
            } else {
                let freq = trainer.root.get_line_freq(input_params[1].to_string(), &trainer.range_manager, hand_order_map);
                println!("{}",freq);
            }
        },
        None => println!("ERROR: Built tree not found"),
    };
}

fn calc_eq_node(input_params: &Vec<&str>, trainer_option: &Option<Trainer>) {
    match trainer_option {
        Some(trainer) => {
            if input_params.len() < 3 {
                println!("ERROR: {} incorrect or missing argument", input_params[0]);
            } else {
                let oop = if input_params[1] == "oop" || input_params[1] == "OOP" {
                    true
                } else if input_params[1] == "ip" || input_params[1] == "IP" {
                    false
                } else {
                    println!("ERROR: {} incorrect or missing argument", input_params[0]);
                    return;
                };
                let equities = vec![0.0; 1326];
                let matchups = vec![0.0; 1326];
                let total = 0.0;
                for el in &equities {
                     print!("{} ", el);
                }
                println!("");
                for el in &matchups {
                     print!("{} ", el);
                }
                println!("");
                println!("{}",total);
            }
        },
        None => println!("ERROR: Built tree not found"),
    };
}

fn calc_ev(input_params: &Vec<&str>, trainer_option: &Option<Trainer>) {
    match trainer_option {
        Some(trainer) => {
            if input_params.len() < 3 {
                println!("ERROR: {} incorrect or missing argument", input_params[0]);
            } else {
                let oop = if input_params[1] == "oop" || input_params[1] == "OOP" {
                    true
                } else if input_params[1] == "ip" || input_params[1] == "IP" {
                    false
                } else {
                    println!("ERROR: {} incorrect or missing argument", input_params[0]);
                    return;
                };
                let equities = vec![0.0; 1326];
                let matchups = vec![0.0; 1326];
                for el in &equities {
                     print!("{} ", el);
                }
                println!("");
                for el in &matchups {
                     print!("{} ", el);
                }
                println!("");
            }
        },
        None => println!("ERROR: Built tree not found"),
    };
}

fn show_node(input_params: &Vec<&str>, trainer_option: &Option<Trainer>) {
    match trainer_option {
        Some(trainer) => {
            if input_params.len() == 1 {
                println!("ERROR: {} incorrect or missing argument", input_params[0]);
            } else {
                let node_info = trainer.root.get_node(input_params[1].to_string(), &trainer.range_manager);
                println!("{}",node_info);
            }
        },
        None => println!("ERROR: Built tree not found"),
    };
}

fn add_line(input_params: &Vec<&str>, tree_information: &mut TreeInformation) {
    if input_params.len() == 1 {
        println!("ERROR: {} incorrect or missing argument", input_params[0]);
    } else {
        let mut line = vec![];
        for i in 1..input_params.len() {
            if input_params[i] == "" {
                continue;
            }
            if input_params[i].parse::<u32>().is_ok() == false {
                println!("ERROR: Invalid value");
                break;
            }
            line.push(input_params[i].parse::<u32>().unwrap());
        }
        if let Some(ref mut lines) = tree_information.lines {
            lines.push(line);
        } else {
            tree_information.lines = Some(vec![line]);
        }
        println!("{} ok!", input_params[0]);
    }
}

fn clear_lines(tree_information: &mut TreeInformation) {
    tree_information.lines = None;
    println!("clear_lines ok!");
}

fn build_tree(tree_information: &mut TreeInformation, trainer: &mut Option<Trainer>) {
    if tree_information.eff_stack.is_none() || tree_information.pot.is_none() || tree_information.oop_range.is_none() || tree_information.ip_range.is_none() || tree_information.lines.is_none() || tree_information.board.is_none() {
        println!("ERROR: build_tree missing/incorrect tree");
    } else {
        let mut range_manager = RangeManager::new(tree_information.oop_range.as_ref().unwrap().clone(), tree_information.ip_range.as_ref().unwrap().clone(), tree_information.board.as_ref().unwrap().clone());
        let mut new_trainer = Trainer::new(range_manager, tree_information.lines.as_ref().unwrap().clone(), tree_information.eff_stack.unwrap(), tree_information.pot.unwrap());
        *trainer = Some(new_trainer);
        println!("build_tree ok!");
    }
}

fn go(input_params: &Vec<&str>, trainer_option: &mut Option<Trainer>, accuracy: &Accuracy, end_string: &String) {
    match trainer_option {
        Some(trainer) => {
            let train_finish = if input_params.len() == 1 || (input_params.len() == 2 && input_params[1] == "") {
                Some(TrainFinish::Indefinite)
            } else if input_params.len() > 2 && input_params[1].parse::<u64>().is_ok() == true && (input_params[2] == "seconds" || input_params[2] == "steps") {
                if input_params[2] == "seconds" {
                    Some(TrainFinish::Seconds(input_params[1].parse::<u64>().unwrap()))
                } else {
                    Some(TrainFinish::Iterations(input_params[1].parse::<u64>().unwrap()))
                }
            } else {
                None
            };
            if let Some(x) = train_finish {
                println!("SOLVER: started");
                println!("{} ok!", input_params[0]);
                if end_string.len() > 0 {
                    println!("{}",end_string);
                }
                trainer.train(accuracy, x);
                println!("SOLVER: stopped (required accuracy reached)");
            }
        },
        None => {
            println!("ERROR: Built tree not found");
            if end_string.len() > 0 {
                println!("{}",end_string);
            }
        },
        
    };
}