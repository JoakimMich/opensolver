// Suit isomorphism at chance nodes.
//
// Two suits are interchangeable at a chance node when swapping them changes neither the board nor
// any earlier board of the hand (so the whole history is symmetric), nor the players' starting
// ranges. Then hands h and swap(h) reach the chance node with the same probabilities, and the
// dealt cards pair up into games that are identical up to the suit swap. Only one card per pair
// is solved: the value of hand h after the skipped card is the value of swap(h) after the dealt
// one.
//
// Checking only the current board is not enough: on QsJh2h + Qd, spades and diamonds hold the same
// ranks, but the flop had Qs and not Qd, so AsKs and AdKd reach the turn differently.

use crate::hand_range::{Combo, HandRange};
use crate::range::HashMap;

/// A card that is not dealt: it plays like `deck[canonical_index]` with suits `swap` exchanged
#[derive(Debug, Clone)]
pub struct IsomorphicCard {
    pub card: u8,
    pub canonical_index: usize,
    pub swap: (u8, u8),
}

/// The cards dealt at a chance node, and how the skipped cards are derived from them
#[derive(Debug, Default)]
pub struct BoardIsomorphism {
    /// Cards that are dealt (children of the chance node, in this order)
    pub deck: Vec<u8>,
    /// Cards that are not dealt
    pub skipped: Vec<IsomorphicCard>,
    /// For each dealt card, the permutations (index into `oop_perms`/`ip_perms`) of the cards
    /// that are derived from it
    pub skipped_by_canonical: Vec<Vec<usize>>,
    /// perm[h] = index of the suit-swapped hand h in the range on this board, one per skipped card
    pub oop_perms: Vec<Vec<u16>>,
    pub ip_perms: Vec<Vec<u16>>,
}

#[inline]
pub fn swap_suit(card: u8, swap: (u8, u8)) -> u8 {
    let suit = card & 3;
    if suit == swap.0 {
        card - swap.0 + swap.1
    } else if suit == swap.1 {
        card - swap.1 + swap.0
    } else {
        card
    }
}

fn combo_key(c0: u8, c1: u8) -> (u8, u8) {
    (c0.min(c1), c0.max(c1))
}

/// Ranks held in each suit
fn suit_ranks(board_mask: u64) -> [u16; 4] {
    let mut ranks = [0u16; 4];
    for card in 0..52u8 {
        if board_mask & (1u64 << card) != 0 {
            ranks[(card & 3) as usize] |= 1 << (card >> 2);
        }
    }
    ranks
}

/// perm[h] = index of the suit-swapped hand h, or None if the range is not symmetric
fn swap_permutation(hands: &[Combo], swap: (u8, u8)) -> Option<Vec<u16>> {
    let index: HashMap<(u8, u8), usize> = hands.iter().enumerate().map(|(i, c)| (combo_key(c.0, c.1), i)).collect();
    hands.iter().map(|c| {
        let j = *index.get(&combo_key(swap_suit(c.0, swap), swap_suit(c.1, swap)))?;
        if hands[j].2 == c.2 { Some(j as u16) } else { None }
    }).collect()
}

/// Splits the cards that can be dealt to `board_mask` into dealt and skipped cards.
/// `earlier_boards` are the boards of the previous streets, `starting_ranges` the ranges on the
/// initial board and `oop_range`/`ip_range` the ranges on this board.
pub fn board_isomorphism(board_mask: u64, earlier_boards: &[u64], starting_ranges: (&HandRange, &HandRange), oop_range: &HandRange, ip_range: &HandRange, enabled: bool) -> BoardIsomorphism {
    let boards_ranks: Vec<[u16; 4]> = earlier_boards.iter().chain(std::iter::once(&board_mask)).map(|&board| suit_ranks(board)).collect();

    // Each suit's class representative (lowest interchangeable suit)
    let mut representative = [0u8, 1, 2, 3];
    if enabled {
        for suit in 1..4u8 {
            for candidate in 0..suit {
                if representative[candidate as usize] != candidate || boards_ranks.iter().any(|ranks| ranks[suit as usize] != ranks[candidate as usize]) {
                    continue;
                }
                let swap = (candidate, suit);
                if swap_permutation(&starting_ranges.0.hands, swap).is_some() && swap_permutation(&starting_ranges.1.hands, swap).is_some() {
                    representative[suit as usize] = candidate;
                    break;
                }
            }
        }
    }

    let mut iso = BoardIsomorphism::default();
    let mut deck_index = [usize::MAX; 52];
    for card in 0..52u8 {
        if board_mask & (1u64 << card) == 0 && representative[(card & 3) as usize] == card & 3 {
            deck_index[card as usize] = iso.deck.len();
            iso.deck.push(card);
        }
    }
    iso.skipped_by_canonical = vec![vec![]; iso.deck.len()];

    let mut perm_ids: HashMap<(u8, u8), usize> = HashMap::default();
    for card in 0..52u8 {
        let suit = card & 3;
        let rep = representative[suit as usize];
        if board_mask & (1u64 << card) != 0 || rep == suit {
            continue;
        }
        let swap = (rep, suit);
        let canonical_index = deck_index[swap_suit(card, swap) as usize];
        let perm_id = *perm_ids.entry(swap).or_insert_with(|| {
            iso.oop_perms.push(swap_permutation(&oop_range.hands, swap).unwrap());
            iso.ip_perms.push(swap_permutation(&ip_range.hands, swap).unwrap());
            iso.oop_perms.len() - 1
        });
        iso.skipped_by_canonical[canonical_index].push(perm_id);
        iso.skipped.push(IsomorphicCard { card, canonical_index, swap });
    }

    iso
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::get_card_mask;

    fn isomorphism(board: &str, oop: &str, ip: &str) -> BoardIsomorphism {
        let board_mask = get_card_mask(board);
        let mut oop = HandRange::from_string(oop.to_string());
        let mut ip = HandRange::from_string(ip.to_string());
        oop.remove_conflicting_combos(board_mask);
        ip.remove_conflicting_combos(board_mask);
        board_isomorphism(board_mask, &[], (&oop, &ip), &oop, &ip, true)
    }

    #[test]
    fn rainbow_flop_has_none() {
        let iso = isomorphism("Ks7d2h", "AA,KQs,T9", "QQ,AK");
        assert_eq!(iso.deck.len(), 49);
        assert!(iso.skipped.is_empty());
    }

    #[test]
    fn two_tone_flop_swaps_the_missing_suits() {
        // d and c are both absent, so clubs play like diamonds
        let iso = isomorphism("QsJh2h", "AA,KQs,T9", "QQ,AK");
        assert_eq!(iso.deck.len(), 36);
        assert_eq!(iso.skipped.len(), 13);
        assert!(iso.skipped.iter().all(|c| c.card & 3 == 3 && c.swap == (2, 3)));
        assert_eq!(iso.deck[iso.skipped[0].canonical_index], iso.skipped[0].card - 1);
    }

    #[test]
    fn monotone_flop_swaps_three_suits() {
        let iso = isomorphism("QhJh2h", "AA,KQs,T9", "QQ,AK");
        assert_eq!(iso.deck.len(), 23);
        assert_eq!(iso.skipped.len(), 26);
    }

    #[test]
    fn paired_suits_with_equal_ranks() {
        // 7s and 7d: spades and diamonds hold the same ranks
        let iso = isomorphism("7s7dKh", "AA,KQs,T9", "QQ,AK");
        assert!(iso.skipped.iter().all(|c| c.swap == (0, 2)));
        assert_eq!(iso.skipped.len(), 12);
    }

    #[test]
    fn earlier_boards_must_be_symmetric_too() {
        // QsJh2hQd: spades and diamonds both hold a queen, but only the turn made them equal
        let turn = get_card_mask("QsJh2hQd");
        let mut range = HandRange::from_string("AA,KQs,T9".to_string());
        range.remove_conflicting_combos(turn);
        let iso = board_isomorphism(turn, &[get_card_mask("QsJh2h")], (&range, &range), &range, &range, true);
        assert!(iso.skipped.is_empty());
        // as a turn start the history is only this board, so the swap is fine
        let iso = board_isomorphism(turn, &[], (&range, &range), &range, &range, true);
        assert_eq!(iso.skipped.len(), 12);
    }

    #[test]
    fn asymmetric_range_disables_swap() {
        let iso = isomorphism("QsJh2h", "AA,KdQd", "QQ,AK");
        assert!(iso.skipped.is_empty());
        let iso = isomorphism("QsJh2h", "AA,KdQd,KcQc", "QQ,AK");
        assert_eq!(iso.skipped.len(), 13);
    }

    #[test]
    fn permutation_maps_to_swapped_hand() {
        let board_mask = get_card_mask("QsJh2h");
        let mut range = HandRange::from_string("AA,KQs,T9".to_string());
        range.remove_conflicting_combos(board_mask);
        let iso = board_isomorphism(board_mask, &[], (&range, &range), &range, &range, true);
        let perm = &iso.oop_perms[0];
        for (i, combo) in range.hands.iter().enumerate() {
            let swapped = &range.hands[perm[i] as usize];
            assert_eq!(combo_key(swapped.0, swapped.1), combo_key(swap_suit(combo.0, (2, 3)), swap_suit(combo.1, (2, 3))));
        }
    }
}
