use once_cell::sync::Lazy;
use regex::Regex;
use std::mem;
use std::str::FromStr;

/// A struct representing a player's 13x13 range.
///
/// # Examples
/// ```
/// use postflop_solver::*;
///
/// // construct a range from a string
/// let range = "QQ+,AKs".parse::<Range>().unwrap();
///
/// // rank is defined as follows: A => 12, K => 11, ..., 2 => 0
/// let ace_rank = 12;
/// let king_rank = 11;
/// let queen_rank = 10;
///
/// // check that the hand "QQ" is in the range
/// assert_eq!(range.get_prob_pair(queen_rank), 1.0);
///
/// // check that the hand "AKo" is not in the range
/// assert_eq!(range.get_prob_offsuit(ace_rank, king_rank), 0.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    data: [f32; 52 * 51 / 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suitedness {
    Suited,
    Offsuit,
    All,
    Specific(u8, u8),
}

const COMBO_PAT: &str = r"(?:(?:[AKQJT2-9]{2}[os]?)|(?:(?:[AKQJT2-9][cdhs]){2}))";
const PROB_PAT: &str = r"(?:(?:[01](\.\d*)?)|(?:\.\d+))";

static RANGE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^(?P<range>{COMBO_PAT}(?:\+|(?:-{COMBO_PAT}))?)(?::(?P<prob>{PROB_PAT}))?$"
    ))
    .unwrap()
});

static TRIM_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s*([-:,])\s*").unwrap());

/// Returns an index of the given card pair.
///
/// Input card ID: 2c2d2h2s => `0-3`, 3c3d3h3s => `4-7`, ..., AcAdAhAs => `48-51`.
///
/// # Examples
/// ```
/// use postflop_solver::*;
///
/// assert_eq!(card_pair_index(0, 1), 0);       // '2c2d' is the first card pair
/// assert_eq!(card_pair_index(50, 51), 1325);  // 'AhAs' is the last card pair
/// ```
#[inline]
pub fn card_pair_index(mut card1: u8, mut card2: u8) -> usize {
    if card1 > card2 {
        mem::swap(&mut card1, &mut card2);
    }
    card1 as usize * (101 - card1 as usize) / 2 + card2 as usize - 1
}

#[inline]
fn pair_indices(rank: u8) -> Vec<usize> {
    let mut result = Vec::with_capacity(6);
    for i in 0..4 {
        for j in i + 1..4 {
            result.push(card_pair_index(4 * rank + i, 4 * rank + j));
        }
    }
    result
}

#[inline]
fn nonpair_indices(rank1: u8, rank2: u8) -> Vec<usize> {
    debug_assert!(rank1 != rank2);
    let mut result = Vec::with_capacity(16);
    for i in 0..4 {
        for j in 0..4 {
            result.push(card_pair_index(4 * rank1 + i, 4 * rank2 + j));
        }
    }
    result
}

#[inline]
fn suited_indices(rank1: u8, rank2: u8) -> Vec<usize> {
    debug_assert!(rank1 != rank2);
    let mut result = Vec::with_capacity(4);
    for i in 0..4 {
        result.push(card_pair_index(4 * rank1 + i, 4 * rank2 + i));
    }
    result
}

#[inline]
fn offsuit_indices(rank1: u8, rank2: u8) -> Vec<usize> {
    debug_assert!(rank1 != rank2);
    let mut result = Vec::with_capacity(12);
    for i in 0..4 {
        for j in 0..4 {
            if i != j {
                result.push(card_pair_index(4 * rank1 + i, 4 * rank2 + j));
            }
        }
    }
    result
}

#[inline]
fn indices_with_suitedness(rank1: u8, rank2: u8, suitedness: Suitedness) -> Vec<usize> {
    if rank1 == rank2 {
        match suitedness {
            Suitedness::All => pair_indices(rank1),
            Suitedness::Specific(suit1, suit2) => {
                vec![card_pair_index(4 * rank1 + suit1, 4 * rank1 + suit2)]
            }
            _ => panic!("invalid suitedness with a pair"),
        }
    } else {
        match suitedness {
            Suitedness::Suited => suited_indices(rank1, rank2),
            Suitedness::Offsuit => offsuit_indices(rank1, rank2),
            Suitedness::All => nonpair_indices(rank1, rank2),
            Suitedness::Specific(suit1, suit2) => {
                vec![card_pair_index(4 * rank1 + suit1, 4 * rank2 + suit2)]
            }
        }
    }
}

/// Attempts to convert a rank character to a rank index.
///
/// `'A'` => `12`, `'K'` => `11`, ..., `'2'` => `0`.
#[inline]
pub fn char_to_rank(c: char) -> Result<u8, String> {
    match c {
        'A' => Ok(12),
        'K' => Ok(11),
        'Q' => Ok(10),
        'J' => Ok(9),
        'T' => Ok(8),
        '2'..='9' => Ok(c as u8 - b'2'),
        _ => Err(format!("invalid input: {c}")),
    }
}

/// Attempts to conver a suit character to a suit index.
///
/// `'c'` => `0`, `'d'` => `1`, `'h'` => `2`, `'s'` => `3`.
#[inline]
pub fn char_to_suit(c: char) -> Result<u8, String> {
    match c {
        'c' => Ok(0),
        'd' => Ok(1),
        'h' => Ok(2),
        's' => Ok(3),
        _ => Err(format!("invalid input: {c}")),
    }
}

/// Attempts to convert a rank index to a rank character.
///
/// `12` => `'A'`, `11` => `'K'`, ..., `0` => `'2'`.
#[inline]
pub fn rank_to_char(rank: u8) -> Result<char, String> {
    match rank {
        12 => Ok('A'),
        11 => Ok('K'),
        10 => Ok('Q'),
        9 => Ok('J'),
        8 => Ok('T'),
        0..=7 => Ok((rank + b'2') as char),
        _ => Err(format!("invalid input: {rank}")),
    }
}

/// Attempts to convert a suit index to a suit character.
///
/// `0` => `'c'`, `1` => `'d'`, `2` => `'h'`, `3` => `'s'`.
#[inline]
pub fn suit_to_char(suit: u8) -> Result<char, String> {
    match suit {
        0 => Ok('c'),
        1 => Ok('d'),
        2 => Ok('h'),
        3 => Ok('s'),
        _ => Err(format!("invalid input: {suit}")),
    }
}

#[inline]
fn parse_singleton(combo: &str) -> Result<(u8, u8, Suitedness), String> {
    if combo.len() == 4 {
        parse_simple_singleton(combo)
    } else {
        parse_compound_singleton(combo)
    }
}

#[inline]
fn parse_simple_singleton(combo: &str) -> Result<(u8, u8, Suitedness), String> {
    let mut chars = combo.chars();
    let rank1 = char_to_rank(chars.next().unwrap())?;
    let suit1 = char_to_suit(chars.next().unwrap())?;
    let rank2 = char_to_rank(chars.next().unwrap())?;
    let suit2 = char_to_suit(chars.next().unwrap())?;
    if rank1 < rank2 {
        return Err(format!(
            "First rank must be equal or higher than second rank: {combo}"
        ));
    }
    if rank1 == rank2 && suit1 == suit2 {
        return Err(format!("Duplicate cards are not allowed: {combo}"));
    }
    Ok((rank1, rank2, Suitedness::Specific(suit1, suit2)))
}

#[inline]
fn parse_compound_singleton(combo: &str) -> Result<(u8, u8, Suitedness), String> {
    let mut chars = combo.chars();
    let rank1 = char_to_rank(chars.next().unwrap())?;
    let rank2 = char_to_rank(chars.next().unwrap())?;
    let suitedness = chars.next().map_or(Suitedness::All, |c| match c {
        's' => Suitedness::Suited,
        'o' => Suitedness::Offsuit,
        _ => panic!("parse_singleton: invalid suitedness: {combo}"),
    });
    if rank1 < rank2 {
        return Err(format!(
            "First rank must be equal or higher than second rank: {combo}"
        ));
    }
    if rank1 == rank2 && suitedness != Suitedness::All {
        return Err(format!("Pair with suitedness is not allowed: {combo}"));
    }
    Ok((rank1, rank2, suitedness))
}

#[inline]
fn check_card(card: u8) -> Result<(), String> {
    if card < 52 {
        Ok(())
    } else {
        Err(format!("Invalid card: {card}"))
    }
}

#[inline]
fn check_rank(rank: u8) -> Result<(), String> {
    if rank < 13 {
        Ok(())
    } else {
        Err(format!("Invalid rank: {rank}"))
    }
}

#[inline]
fn check_prob(prob: f32) -> Result<(), String> {
    if (0.0..=1.0).contains(&prob) {
        Ok(())
    } else {
        Err(format!("Invalid probability: {prob}"))
    }
}

impl Default for Range {
    #[inline]
    fn default() -> Self {
        Self {
            data: [0.0; 52 * 51 / 2],
        }
    }
}

impl Range {
    /// Creates an empty range.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a full range.
    #[inline]
    pub fn ones() -> Self {
        Self {
            data: [1.0; 52 * 51 / 2],
        }
    }

    /// Creates a range from raw data.
    #[inline]
    pub fn from_raw(data: &[f32; 52 * 51 / 2]) -> Self {
        Self {
            data: data.to_owned(),
        }
    }

    /// Obtains the probability by card indices.
    ///
    /// Input card ID: 2c2d2h2s => `0-3`, 3c3d3h3s => `4-7`, ..., AcAdAhAs => `48-51`.
    #[inline]
    pub fn get_prob_by_cards(&self, card1: u8, card2: u8) -> f32 {
        self.data[card_pair_index(card1, card2)]
    }

    /// Obtains the average probability of a pair.
    #[inline]
    pub fn get_prob_pair(&self, rank: u8) -> f32 {
        self.get_average_prob(&pair_indices(rank))
    }

    /// Obtains the average probability of a suited hand.
    #[inline]
    pub fn get_prob_suited(&self, rank1: u8, rank2: u8) -> f32 {
        self.get_average_prob(&suited_indices(rank1, rank2))
    }

    /// Obtains the average probability of an offsuit hand.
    #[inline]
    pub fn get_prob_offsuit(&self, rank1: u8, rank2: u8) -> f32 {
        self.get_average_prob(&offsuit_indices(rank1, rank2))
    }

    /// Sets the probability by card indices.
    ///
    /// Input card ID: 2c2d2h2s => `0-3`, 3c3d3h3s => `4-7`, ..., AcAdAhAs => `48-51`.
    #[inline]
    pub fn set_prob_by_cards(&mut self, card1: u8, card2: u8, prob: f32) -> Result<(), String> {
        check_card(card1)?;
        check_card(card2)?;
        self.data[card_pair_index(card1, card2)] = prob;
        Ok(())
    }

    /// Sets the probability of a pair.
    #[inline]
    pub fn set_prob_pair(&mut self, rank: u8, prob: f32) -> Result<(), String> {
        check_rank(rank)?;
        check_prob(prob)?;
        self.set_prob(&pair_indices(rank), prob);
        Ok(())
    }

    /// Sets the probability of a suited hand.
    #[inline]
    pub fn set_prob_suited(&mut self, rank1: u8, rank2: u8, prob: f32) -> Result<(), String> {
        check_rank(rank1)?;
        check_rank(rank2)?;
        check_prob(prob)?;
        if rank1 == rank2 {
            return Err(format!(
                "set_prob_suited() accepts non-pairs, but got rank1 = rank2 = {rank1}"
            ));
        }
        self.set_prob(&suited_indices(rank1, rank2), prob);
        Ok(())
    }

    /// Sets the probability of an offsuit hand.
    #[inline]
    pub fn set_prob_offsuit(&mut self, rank1: u8, rank2: u8, prob: f32) -> Result<(), String> {
        check_rank(rank1)?;
        check_rank(rank2)?;
        check_prob(prob)?;
        if rank1 == rank2 {
            return Err(format!(
                "set_prob_offsuit() accepts non-pairs, but got rank1 = rank2 = {rank1}"
            ));
        }
        self.set_prob(&offsuit_indices(rank1, rank2), prob);
        Ok(())
    }

    /// Returns whether the range is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.iter().all(|el| *el == 0.0)
    }

    /// Returns whether the two suits are isomorphic.
    #[inline]
    pub fn is_suit_isomorphic(&self, suit1: u8, suit2: u8) -> bool {
        let replace_suit = |suit| {
            if suit == suit1 {
                suit2
            } else if suit == suit2 {
                suit1
            } else {
                suit
            }
        };

        for card1 in 0..52 {
            for card2 in card1 + 1..52 {
                let card1_replaced = (card1 & !3) | replace_suit(card1 & 3);
                let card2_replaced = (card2 & !3) | replace_suit(card2 & 3);
                let prob = self.get_prob_by_cards(card1, card2);
                let prob_replaced = self.get_prob_by_cards(card1_replaced, card2_replaced);
                if (prob - prob_replaced).abs() >= 1e-4 {
                    return false;
                }
            }
        }

        true
    }

    #[inline]
    fn is_same_prob(&self, indices: &[usize]) -> bool {
        let prob = self.data[indices[0]];
        indices.iter().all(|i| (self.data[*i] - prob).abs() < 1e-4)
    }

    #[inline]
    fn get_average_prob(&self, indices: &[usize]) -> f32 {
        let mut sum = 0.0;
        for i in indices {
            sum += self.data[*i] as f64;
        }
        (sum / indices.len() as f64) as f32
    }

    #[inline]
    fn set_prob(&mut self, indices: &[usize], prob: f32) {
        for i in indices {
            self.data[*i] = prob;
        }
    }

    #[inline]
    fn update_with_singleton(&mut self, combo: &str, prob: f32) -> Result<(), String> {
        let (rank1, rank2, suitedness) = parse_singleton(combo)?;
        self.set_prob(&indices_with_suitedness(rank1, rank2, suitedness), prob);
        Ok(())
    }

    #[inline]
    fn update_with_plus_range(&mut self, range: &str, prob: f32) -> Result<(), String> {
        debug_assert!(range.ends_with('+'));
        let lowest_combo = &range[..range.len() - 1];
        let (rank1, rank2, suitedness) = parse_singleton(lowest_combo)?;
        let gap = rank1 - rank2;
        if gap <= 1 {
            // pair and connector (e.g.,  88+, T9s+)
            for i in rank1..13 {
                self.set_prob(&indices_with_suitedness(i, i - gap, suitedness), prob);
            }
        } else {
            // otherwise (e.g., ATo+)
            for i in rank2..rank1 {
                self.set_prob(&indices_with_suitedness(rank1, i, suitedness), prob);
            }
        }
        Ok(())
    }

    #[inline]
    fn update_with_dash_range(&mut self, range: &str, prob: f32) -> Result<(), String> {
        let combo_pair = range.split('-').collect::<Vec<_>>();
        debug_assert!(combo_pair.len() == 2);
        let (rank11, rank12, suitedness) = parse_singleton(combo_pair[0])?;
        let (rank21, rank22, suitedness2) = parse_singleton(combo_pair[1])?;
        let gap = rank11 - rank12;
        let gap2 = rank21 - rank22;
        if suitedness != suitedness2 {
            Err(format!("Suitedness does not match: {range}"))
        } else if gap == gap2 && rank11 > rank21 {
            // same gap (e.g., 88-55, KQo-JTo)
            for i in rank21..=rank11 {
                self.set_prob(&indices_with_suitedness(i, i - gap, suitedness), prob);
            }
            Ok(())
        } else if rank11 == rank21 && rank12 > rank22 {
            // same first rank (e.g., A5s-A2s)
            for i in rank22..=rank12 {
                self.set_prob(&indices_with_suitedness(rank11, i, suitedness), prob);
            }
            Ok(())
        } else {
            Err(format!("Invalid range: {range}"))
        }
    }

    #[inline]
    fn pairs_strings(&self, result: &mut Vec<String>) {
        let mut start: Option<(u8, f32)> = None;

        for i in (-1..13).rev() {
            let rank = i as u8;
            let prev_rank = (i + 1) as u8;

            if start.is_some()
                && (i == -1
                    || !self.is_same_prob(&pair_indices(rank))
                    || start.unwrap().1 != self.get_prob_pair(rank))
            {
                let (start_rank, prob) = start.unwrap();
                let s = rank_to_char(start_rank).unwrap();
                let e = rank_to_char(prev_rank).unwrap();
                let mut tmp = if start_rank == prev_rank {
                    format!("{s}{s}")
                } else if start_rank == 12 {
                    format!("{e}{e}+")
                } else {
                    format!("{s}{s}-{e}{e}")
                };
                if prob != 1.0 {
                    tmp += &format!(":{prob}");
                }
                result.push(tmp);
                start = None;
            }

            if i >= 0
                && self.is_same_prob(&pair_indices(rank))
                && self.get_prob_pair(rank) > 0.0
                && start.is_none()
            {
                start = Some((rank, self.get_prob_pair(rank)));
            }
        }
    }

    #[inline]
    fn nonpairs_strings(&self, result: &mut Vec<String>) {
        for rank1 in (1..13).rev() {
            if self.can_unsuit(rank1) {
                self.high_cards_strings(result, rank1, Suitedness::All);
            } else {
                self.high_cards_strings(result, rank1, Suitedness::Suited);
                self.high_cards_strings(result, rank1, Suitedness::Offsuit);
            }
        }
    }

    fn can_unsuit(&self, rank1: u8) -> bool {
        for rank2 in 0..rank1 {
            let same_suited = self.is_same_prob(&suited_indices(rank1, rank2));
            let same_offsuit = self.is_same_prob(&offsuit_indices(rank1, rank2));
            let prob_suited = self.get_prob_suited(rank1, rank2);
            let prob_offsuit = self.get_prob_offsuit(rank1, rank2);
            if (same_suited && same_offsuit && prob_suited != prob_offsuit)
                || (same_suited != same_offsuit && prob_suited > 0.0 && prob_offsuit > 0.0)
            {
                return false;
            }
        }
        true
    }

    #[inline]
    fn high_cards_strings(&self, result: &mut Vec<String>, rank1: u8, suitedness: Suitedness) {
        let rank1_char = rank_to_char(rank1).unwrap();
        let mut start: Option<(u8, f32)> = None;
        let (getter, suit_char): (fn(u8, u8) -> Vec<usize>, &str) = match suitedness {
            Suitedness::Suited => (suited_indices, "s"),
            Suitedness::Offsuit => (offsuit_indices, "o"),
            Suitedness::All => (nonpair_indices, ""),
            _ => panic!("high_cards_strings: invalid suitedness"),
        };

        for i in (-1..rank1 as i32).rev() {
            let rank2 = i as u8;
            let prev_rank2 = (i + 1) as u8;

            if start.is_some()
                && (i == -1
                    || !self.is_same_prob(&getter(rank1, rank2))
                    || start.unwrap().1 != self.get_average_prob(&getter(rank1, rank2)))
            {
                let (start_rank2, prob) = start.unwrap();
                let s = rank_to_char(start_rank2).unwrap();
                let e = rank_to_char(prev_rank2).unwrap();
                let mut tmp = if start_rank2 == prev_rank2 {
                    format!("{rank1_char}{s}{suit_char}")
                } else if start_rank2 == rank1 - 1 {
                    format!("{rank1_char}{e}{suit_char}+")
                } else {
                    format!("{rank1_char}{s}{suit_char}-{rank1_char}{e}{suit_char}")
                };
                if prob != 1.0 {
                    tmp += &format!(":{prob}");
                }
                result.push(tmp);
                start = None;
            }

            if i >= 0
                && self.is_same_prob(&getter(rank1, rank2))
                && self.get_average_prob(&getter(rank1, rank2)) > 0.0
                && start.is_none()
            {
                start = Some((rank2, self.get_average_prob(&getter(rank1, rank2))));
            }
        }
    }

    #[inline]
    fn suit_specified_strings(&self, result: &mut Vec<String>) {
        // pairs
        for rank in (0..13).rev() {
            if !self.is_same_prob(&pair_indices(rank)) {
                for suit1 in 0..4 {
                    for suit2 in suit1 + 1..4 {
                        let prob = self.get_prob_by_cards(4 * rank + suit1, 4 * rank + suit2);
                        if prob > 0.0 {
                            let mut tmp = format!(
                                "{rank}{suit1}{rank}{suit2}",
                                rank = rank_to_char(rank).unwrap(),
                                suit1 = suit_to_char(suit1).unwrap(),
                                suit2 = suit_to_char(suit2).unwrap(),
                            );
                            if prob != 1.0 {
                                tmp += &format!(":{prob}");
                            }
                            result.push(tmp);
                        }
                    }
                }
            }
        }

        // non-pairs
        for rank1 in (0..13).rev() {
            for rank2 in (0..rank1).rev() {
                // suited
                if !self.is_same_prob(&suited_indices(rank1, rank2)) {
                    for suit in 0..4 {
                        let prob = self.get_prob_by_cards(4 * rank1 + suit, 4 * rank2 + suit);
                        if prob > 0.0 {
                            let mut tmp = format!(
                                "{rank1}{suit}{rank2}{suit}",
                                rank1 = rank_to_char(rank1).unwrap(),
                                rank2 = rank_to_char(rank2).unwrap(),
                                suit = suit_to_char(suit).unwrap(),
                            );
                            if prob != 1.0 {
                                tmp += &format!(":{prob}");
                            }
                            result.push(tmp);
                        }
                    }
                }

                // offsuit
                if !self.is_same_prob(&offsuit_indices(rank1, rank2)) {
                    for suit1 in 0..4 {
                        for suit2 in 0..4 {
                            if suit1 != suit2 {
                                let prob =
                                    self.get_prob_by_cards(4 * rank1 + suit1, 4 * rank2 + suit2);
                                if prob > 0.0 {
                                    let mut tmp = format!(
                                        "{rank1}{suit1}{rank2}{suit2}",
                                        rank1 = rank_to_char(rank1).unwrap(),
                                        suit1 = suit_to_char(suit1).unwrap(),
                                        rank2 = rank_to_char(rank2).unwrap(),
                                        suit2 = suit_to_char(suit2).unwrap(),
                                    );
                                    if prob != 1.0 {
                                        tmp += &format!(":{prob}");
                                    }
                                    result.push(tmp);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl FromStr for Range {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = TRIM_REGEX.replace_all(s, "$1").trim().to_string();
        let mut ranges = s.split(',').collect::<Vec<_>>();

        // remove last empty element if any
        if ranges.last().unwrap().is_empty() {
            ranges.pop();
        }

        let mut result = Self::new();

        for range in ranges.into_iter().rev() {
            let caps = RANGE_REGEX
                .captures(range)
                .ok_or_else(|| format!("Failed to parse range: {range}"))?;

            let range_str = caps.name("range").unwrap().as_str();
            let prob = caps
                .name("prob")
                .map_or(1.0, |s| s.as_str().parse().unwrap());
            check_prob(prob)?;

            if range_str.contains('-') {
                result.update_with_dash_range(range_str, prob)?;
            } else if range_str.contains('+') {
                result.update_with_plus_range(range_str, prob)?;
            } else {
                result.update_with_singleton(range_str, prob)?;
            }
        }

        Ok(result)
    }
}

impl ToString for Range {
    #[inline]
    fn to_string(&self) -> String {
        let mut result = Vec::new();
        self.pairs_strings(&mut result);
        self.nonpairs_strings(&mut result);
        self.suit_specified_strings(&mut result);
        result.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_regex() {
        let tests = [
            ("AK", Some(("AK", None))),
            ("K9s:.67", Some(("K9s", Some(".67")))),
            ("88+:1.", Some(("88+", Some("1.")))),
            ("98s-65s:0.25", Some(("98s-65s", Some("0.25")))),
            ("AcKh", Some(("AcKh", None))),
            ("8h8s+:.67", Some(("8h8s+", Some(".67")))),
            ("9d8d-6d5d:0.25", Some(("9d8d-6d5d", Some("0.25")))),
            ("ak", None),
            ("AKQ", None),
            ("AK+-AJ", None),
            ("K9s.67", None),
            ("88+:2.0", None),
            ("98s-21s", None),
        ];

        for (s, expected) in tests {
            if let Some((range, prob)) = expected {
                let caps = RANGE_REGEX.captures(s).unwrap();
                assert_eq!(caps.name("range").unwrap().as_str(), range);
                if let Some(prob) = prob {
                    assert_eq!(caps.name("prob").unwrap().as_str(), prob);
                } else {
                    assert!(caps.name("prob").is_none());
                }
            } else {
                assert!(!RANGE_REGEX.is_match(s));
            }
        }
    }

    #[test]
    fn trim_regex() {
        let tests = [
            ("  AK  ", "AK"),
            ("K9s: .67", "K9s:.67"),
            ("88+, AQ+", "88+,AQ+"),
            ("98s - 65s: 0.25", "98s-65s:0.25"),
        ];

        for (s, expected) in tests {
            assert_eq!(TRIM_REGEX.replace_all(s, "$1").trim(), expected);
        }
    }

    #[test]
    fn range_from_str() {
        let pair_plus = "88+".parse::<Range>();
        let pair_plus_equiv = "AA,KK,QQ,JJ,TT,99,88".parse::<Range>();
        assert!(pair_plus.is_ok());
        assert_eq!(pair_plus, pair_plus_equiv);

        let pair_plus_suit = "8s8h+".parse::<Range>();
        let pair_plus_suit_equiv = "AhAs,KhKs,QhQs,JhJs,ThTs,9h9s,8h8s".parse::<Range>();
        assert!(pair_plus_suit.is_ok());
        assert_eq!(pair_plus_suit, pair_plus_suit_equiv);

        let connector_plus = "98s+".parse::<Range>();
        let connector_plus_equiv = "AKs,KQs,QJs,JTs,T9s,98s".parse::<Range>();
        assert!(connector_plus.is_ok());
        assert_eq!(connector_plus, connector_plus_equiv);

        let other_plus = "A8o+".parse::<Range>();
        let other_plus_equiv = "AKo,AQo,AJo,ATo,A9o,A8o".parse::<Range>();
        assert!(other_plus.is_ok());
        assert_eq!(other_plus, other_plus_equiv);

        let pair_dash = "88-55".parse::<Range>();
        let pair_dash_equiv = "88,77,66,55".parse::<Range>();
        assert!(pair_dash.is_ok());
        assert_eq!(pair_dash, pair_dash_equiv);

        let connector_dash = "98s-65s".parse::<Range>();
        let connector_dash_equiv = "98s,87s,76s,65s".parse::<Range>();
        assert!(connector_dash.is_ok());
        assert_eq!(connector_dash, connector_dash_equiv);

        let gapper_dash = "AQo-86o".parse::<Range>();
        let gapper_dash_equiv = "AQo,KJo,QTo,J9o,T8o,97o,86o".parse::<Range>();
        assert!(gapper_dash.is_ok());
        assert_eq!(gapper_dash, gapper_dash_equiv);

        let other_dash = "K5-K2".parse::<Range>();
        let other_dash_equiv = "K5,K4,K3,K2".parse::<Range>();
        assert!(other_dash.is_ok());
        assert_eq!(other_dash, other_dash_equiv);

        let suit_compound = "AhAs-QhQs,JJ".parse::<Range>();
        let suit_compound_equiv = "JJ,AhAs,KhKs,QhQs".parse::<Range>();
        assert!(suit_compound.is_ok());
        assert_eq!(suit_compound, suit_compound_equiv);

        let allow_empty = "".parse::<Range>();
        assert!(allow_empty.is_ok());

        let allow_trailing_comma = "AK,".parse::<Range>();
        assert!(allow_trailing_comma.is_ok());

        let comma_error = "AK,,".parse::<Range>();
        assert!(comma_error.is_err());

        let rank_error = "89".parse::<Range>();
        assert!(rank_error.is_err());

        let pair_error = "AAo".parse::<Range>();
        assert!(pair_error.is_err());

        let prob_error = "AQo:1.1".parse::<Range>();
        assert!(prob_error.is_err());

        let dash_error_1 = "AQo-AQo".parse::<Range>();
        assert!(dash_error_1.is_err());

        let dash_error_2 = "AQo-86s".parse::<Range>();
        assert!(dash_error_2.is_err());

        let dash_error_3 = "AQo-KQo".parse::<Range>();
        assert!(dash_error_3.is_err());

        let dash_error_4 = "K2-K5".parse::<Range>();
        assert!(dash_error_4.is_err());

        let dash_error_5 = "AhAs-QsQh".parse::<Range>();
        assert!(dash_error_5.is_err());

        let data = "85s:0.5".parse::<Range>();
        assert!(data.is_ok());

        let data = data.unwrap();
        assert_eq!(data.get_prob_suited(3, 6), 0.5);
        assert_eq!(data.get_prob_suited(6, 3), 0.5);
        assert_eq!(data.get_prob_offsuit(3, 6), 0.0);
        assert_eq!(data.get_prob_offsuit(6, 3), 0.0);
    }

    #[test]
    fn range_to_string() {
        let tests = [
            ("AA,KK", "KK+"),
            ("KK,QQ", "KK-QQ"),
            ("66-22,TT+", "TT+,66-22"),
            ("AA:0.5, KK:1.0, QQ:1.0, JJ:0.5", "AA:0.5,KK-QQ,JJ:0.5"),
            ("AA,AK,AQ", "AA,AQ+"),
            ("AK,AQ,AJs", "AJs+,AQo+"),
            ("KQ,KT,K9,K8,K6,K5", "KQ,KT-K8,K6-K5"),
            ("AhAs-QhQs,JJ", "JJ,AhAs,KhKs,QhQs"),
            ("KJs+,KQo,KsJh", "KJs+,KQo,KsJh"),
            ("KcQh,KJ", "KJ,KcQh"),
        ];

        for (input, expected) in tests {
            let range = input.parse::<Range>();
            assert!(range.is_ok());
            assert_eq!(range.unwrap().to_string(), expected);
        }
    }
}
