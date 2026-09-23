// Card primitives and hand evaluation.
//
// Cards are indexed 0..52 as 4 * rank + suit, with ranks 2..A = 0..12 and suits s,h,d,c = 0..3.
// A set of cards is a u64 mask with bit `card` set.

pub const CARD_COUNT: u8 = 52;

pub const RANK_TO_CHAR: &[char; 13] = &['2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K', 'A'];

pub const SUIT_TO_CHAR: &[char; 4] = &['s', 'h', 'd', 'c'];

/// Convert lowercase rank char to u8
pub fn char_to_rank(c: char) -> u8 {
    match c {
        'a' => 12,
        'k' => 11,
        'q' => 10,
        'j' => 9,
        't' => 8,
        '9' => 7,
        '8' => 6,
        '7' => 5,
        '6' => 4,
        '5' => 3,
        '4' => 2,
        '3' => 1,
        '2' => 0,
        _ => u8::MAX,
    }
}

/// Convert lowercase suit char to u8
pub fn char_to_suit(c: char) -> u8 {
    match c {
        's' => 0,
        'h' => 1,
        'd' => 2,
        'c' => 3,
        _ => u8::MAX,
    }
}

/// Converts a card string (e.g. "QsJh2h") into a card mask. Returns 0 for invalid input.
pub fn get_card_mask(text: &str) -> u64 {
    let bytes = text.as_bytes();
    if bytes.len() % 2 != 0 {
        return 0;
    }
    let mut cards: u64 = 0;
    for pair in bytes.chunks_exact(2) {
        let rank = char_to_rank(pair[0].to_ascii_lowercase() as char);
        let suit = char_to_suit(pair[1].to_ascii_lowercase() as char);
        if rank == u8::MAX || suit == u8::MAX {
            return 0;
        }
        cards |= 1u64 << (4 * rank + suit);
    }
    cards
}

/// Converts a card mask to its string representation, lowest card first
pub fn mask_to_string(card_mask: u64) -> String {
    let mut card_str = String::new();
    for i in 0..CARD_COUNT {
        if ((1u64 << i) & card_mask) != 0 {
            card_str.push(RANK_TO_CHAR[usize::from(i >> 2)]);
            card_str.push(SUIT_TO_CHAR[usize::from(i & 3)]);
        }
    }
    card_str
}

// Hand strength is (category << 12) | sub-rank, where the sub-rank orders hands within a
// category and always fits in 12 bits. Higher is better and equal values are exact ties.
const HIGH_CARD: u16 = 0;
const PAIR: u16 = 1;
const TWO_PAIR: u16 = 2;
const THREE_OF_A_KIND: u16 = 3;
const STRAIGHT: u16 = 4;
const FLUSH: u16 = 5;
const FULL_HOUSE: u16 = 6;
const FOUR_OF_A_KIND: u16 = 7;
const STRAIGHT_FLUSH: u16 = 8;

const fn binomials() -> [[u16; 6]; 13] {
    let mut table = [[0u16; 6]; 13];
    let mut n = 0;
    while n < 13 {
        table[n][0] = 1;
        let mut k = 1;
        while k < 6 {
            table[n][k] = if n == 0 { 0 } else { table[n - 1][k - 1] + table[n - 1][k] };
            k += 1;
        }
        n += 1;
    }
    table
}

// BINOMIAL[n][k] = n choose k
const BINOMIAL: [[u16; 6]; 13] = binomials();

/// Index of a set of ranks in colexicographic order. For sets of equal size this orders them
/// by highest rank, then next highest, and so on, which is exactly kicker order.
#[inline(always)]
fn colex(mut ranks: u16) -> u16 {
    let mut index = 0;
    let mut k = 1;
    while ranks != 0 {
        index += BINOMIAL[ranks.trailing_zeros() as usize][k];
        ranks &= ranks - 1;
        k += 1;
    }
    index
}

/// Keeps only the `n` highest ranks of the set
#[inline(always)]
fn keep_highest(mut ranks: u16, n: u32) -> u16 {
    while ranks.count_ones() > n {
        ranks &= ranks - 1;
    }
    ranks
}

#[inline(always)]
fn highest(ranks: u16) -> u16 {
    15 - ranks.leading_zeros() as u16
}

/// Highest straight in the rank set as 0 (wheel) ..= 9 (broadway), if any
#[inline(always)]
fn highest_straight(ranks: u16) -> Option<u16> {
    // Bit 0 is the ace playing low, bits 1..=13 are 2..A
    let r = ((ranks as u32) << 1) | ((ranks as u32) >> 12);
    let starts = r & (r >> 1) & (r >> 2) & (r >> 3) & (r >> 4);
    if starts == 0 {
        None
    } else {
        Some(31 - starts.leading_zeros() as u16)
    }
}

/// Evaluates the best 5-card hand from a mask of 5 to 7 cards. Higher is better.
#[inline]
pub fn evaluate(cards: u64) -> u16 {
    let mut suits = [0u16; 4];
    let mut rest = cards;
    while rest != 0 {
        let card = rest.trailing_zeros();
        suits[(card & 3) as usize] |= 1 << (card >> 2);
        rest &= rest - 1;
    }
    let [s0, s1, s2, s3] = suits;

    // With at most 7 cards a flush excludes quads and full houses, so it can be decided first
    for &suited in &suits {
        if suited.count_ones() >= 5 {
            return match highest_straight(suited) {
                Some(top) => (STRAIGHT_FLUSH << 12) | top,
                None => (FLUSH << 12) | colex(keep_highest(suited, 5)),
            };
        }
    }

    // Sets of ranks held at least once / twice / three times / four times
    let any = s0 | s1 | s2 | s3;
    let two_plus = (s0 & s1) | (s2 & s3) | ((s0 | s1) & (s2 | s3));
    let three_plus = (s0 & s1 & (s2 | s3)) | (s2 & s3 & (s0 | s1));
    let quads = s0 & s1 & s2 & s3;
    let trips = three_plus & !quads;
    let pairs = two_plus & !three_plus;

    if quads != 0 {
        let quad = highest(quads);
        let kicker = highest(any & !(1 << quad));
        return (FOUR_OF_A_KIND << 12) | (quad * 13 + kicker);
    }
    if trips != 0 {
        let trip = highest(trips);
        let others = (trips & !(1 << trip)) | pairs;
        if others != 0 {
            return (FULL_HOUSE << 12) | (trip * 13 + highest(others));
        }
    }
    if let Some(top) = highest_straight(any) {
        return (STRAIGHT << 12) | top;
    }
    if trips != 0 {
        let trip = highest(trips);
        let kickers = keep_highest(any & !(1 << trip), 2);
        return (THREE_OF_A_KIND << 12) | (trip * 78 + colex(kickers));
    }
    if pairs.count_ones() >= 2 {
        let high = highest(pairs);
        let low = highest(pairs & !(1 << high));
        let kicker = highest(any & !(1 << high) & !(1 << low));
        return (TWO_PAIR << 12) | (high * 169 + low * 13 + kicker);
    }
    if pairs != 0 {
        let pair = highest(pairs);
        let kickers = keep_highest(any & !(1 << pair), 3);
        return (PAIR << 12) | (pair * 286 + colex(kickers));
    }
    (HIGH_CARD << 12) | colex(keep_highest(any, 5))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn category(value: u16) -> usize {
        (value >> 12) as usize
    }

    #[test]
    fn card_mask_round_trip() {
        assert_eq!(get_card_mask("2s"), 1);
        assert_eq!(get_card_mask("Ac"), 1 << 51);
        assert_eq!(get_card_mask("QsJh2h"), get_card_mask("qsjh2h"));
        assert_eq!(mask_to_string(get_card_mask("QsJh2h")), "2hJhQs");
        assert_eq!(get_card_mask("Qx"), 0);
        assert_eq!(get_card_mask("Qs2"), 0);
    }

    #[test]
    fn hand_order() {
        let hands = [
            "7s5h4d3c2s", "AsKhQdJc9s", "2s2h3d4c5s", "AsAhKdQcJs", "3s3h2d2c4s", "AsAhKdKcQs",
            "2s2h2d3c4s", "AsAhAdKcQs", "As2h3d4c5s", "6s2h3d4c5s", "AsKhQdJcTs", "2s3s4s5s7s",
            "AsKsQsJs9s", "2s2h2d3c3s", "AsAhAdKcKs", "2s2h2d2c3s", "AsAhAdAcKs", "As2s3s4s5s",
            "TsJsQsKsAs",
        ];
        for pair in hands.windows(2) {
            assert!(evaluate(get_card_mask(pair[0])) < evaluate(get_card_mask(pair[1])), "{} < {}", pair[0], pair[1]);
        }
        // Only the best five cards count
        assert_eq!(evaluate(get_card_mask("AsAhKdKcQs2h3h")), evaluate(get_card_mask("AdAcKsKhQc4d5d")));
        assert_eq!(evaluate(get_card_mask("AsAhKdKcQsQh2h")), evaluate(get_card_mask("AsAhKdKcQs")));
    }

    // Every 5-card hand, checked against the known category counts and 7462 distinct values
    #[test]
    fn all_five_card_hands() {
        let mut counts = [0u32; 9];
        let mut seen = vec![false; 1 << 16];
        for a in 0..52 { for b in a + 1..52 { for c in b + 1..52 { for d in c + 1..52 { for e in d + 1..52 {
            let value = evaluate((1u64 << a) | (1u64 << b) | (1u64 << c) | (1u64 << d) | (1u64 << e));
            counts[category(value)] += 1;
            seen[value as usize] = true;
        }}}}}
        assert_eq!(counts, [1302540, 1098240, 123552, 54912, 10200, 5108, 3744, 624, 40]);
        assert_eq!(seen.iter().filter(|&&s| s).count(), 7462);
    }
}
