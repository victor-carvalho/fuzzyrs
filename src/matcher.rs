use super::unicode::next_code_point;

#[derive(Debug)]
pub struct MatchResult {
    matches: Option<Vec<usize>>,
    score: isize,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct MatchOptions {
    pub case_sensitive: bool,
    pub match_position: bool
}

#[derive(Debug)]
enum InputState {
    Beginning,
    InWord,
    InSpace,
    InSpecial,
}

macro_rules! choose {
    ($cond: expr, $iftrue: expr, $iffalse: expr) => {{
        if $cond { $iftrue } else { $iffalse }
    }}
}

#[inline]
fn state_from_char(c: char) -> InputState {
    if c.is_alphanumeric() {
        InputState::InWord
    } else if c.is_whitespace() {
        InputState::InSpace
    } else {
        InputState::InSpecial
    }
}

const SCORE_BEGINNING: isize = 20;
const SCORE_BOUNDARY: isize = 10;
const SCORE_MATCH: isize = 3;
const SCORE_CONSECUTIVE: isize = 3;


#[inline]
fn bonus_at(state: InputState, c: char, distance: usize) -> isize {
    let mut score = match state {
        InputState::Beginning => SCORE_BEGINNING,
        InputState::InSpace => SCORE_BOUNDARY,
        InputState::InSpecial => choose!(c.is_alphanumeric(), SCORE_BOUNDARY, SCORE_MATCH),
        InputState::InWord =>  choose!(!c.is_alphanumeric(), SCORE_BOUNDARY, SCORE_MATCH),
    };
    if distance == 1 {
        score += SCORE_CONSECUTIVE;
    }
    score
}

pub trait Matcher {
    fn match_term(&self, input: &[u8], opts: MatchOptions) -> Option<MatchResult>;
}

#[derive(Debug)]
pub struct FuzzyMatcher {
    term: Vec<char>
}

impl FuzzyMatcher {
    pub fn new(term: &str) -> Self {
        FuzzyMatcher {
            term: term.chars().collect()
        }
    }
}

impl Matcher for FuzzyMatcher {
    fn match_term(&self, input: &[u8], opts: MatchOptions) -> Option<MatchResult> {
        let term = &self.term;
        if term.is_empty() || input.is_empty() {
            return None;
        }

        let mut state = InputState::Beginning;
        
        let mut total_score = 0;
        let mut matches = if opts.match_position {
            vec![0; term.len()]
        } else {
            Vec::new()
        };
        
        let mut term_chars = term.iter().copied();
        
        let mut term_index = 0;
        let mut current = term_chars.next().unwrap();
        
        let mut last_match = 0;
        let mut char_index = 0;
        let mut byte_index = 0;
        while byte_index < input.len() {
            if let Some(c) = next_code_point(&input[byte_index..input.len()]) {
                if c == current {
                    let distance = choose!(term_index != 0, char_index - last_match, 0);
                    total_score += bonus_at(state, c, distance);
                    last_match = char_index;
                    if opts.match_position {
                        matches[term_index] = byte_index;
                    }
                    if let Some(ch) = term_chars.next() {
                        current = ch;
                        term_index += 1;
                    } else {
                        return Some(MatchResult {
                            score: total_score,
                            matches: if opts.match_position {
                                Some(matches)
                            } else {
                                None
                            }
                        });
                    }
                }
                state = state_from_char(c);
                char_index += 1;
                byte_index += c.len_utf8();
            } else {
                char_index += 1;
                byte_index += 1;
            }
        }

        return None;
    }
}

#[derive(Debug)]
pub struct ExactMatcher {
    term: Vec<char>,
    failure_function: Vec<isize>
}

impl ExactMatcher {
    pub fn new(term: &str) -> Self {
        let chars: Vec<char> = term.chars().collect();
        let failure_function = build_failure_function(&chars);
        ExactMatcher {
            term: chars,
            failure_function
        }
    }
}

impl Matcher for ExactMatcher {
    fn match_term(&self, input: &[u8], opts: MatchOptions) -> Option<MatchResult> {
        let mut state = InputState::Beginning;
        let mut i = 0;
        let mut j = 0;

        let mut match_start = 0;
        let mut match_score = 0;
        let mut best_start = 0;
        let mut best_score = 0;

        while i < input.len() {
            if let Some(c) = next_code_point(&input[i..input.len()]) {
                if c == self.term[j] {
                    if j == 0 {
                        match_start = i;
                        match_score = bonus_at(state, c, 0);
                    }
                    i += 1;
                    j += 1;
                    if j == self.term.len() {
                        if match_score > best_score {
                            best_start = match_start;
                            best_score = match_score;
                        }
                        j = self.failure_function[self.term.len()] as usize;
                    }
                } else {
                    if self.failure_function[j] < 0 {
                        i += 1;
                        j = 0;
                    } else {
                        j = self.failure_function[j] as usize;
                    }
                    if j == 0 {
                        match_score = 0;
                    }
                }
                state = state_from_char(c);
            } else {
                i += 1;
                j = 0;
                match_score = 0;
            }
        }
        if best_score > 0 {
            let matches: Option<Vec<usize>> = if opts.match_position {
                let best_end = best_start + self.term.len(); 
                Some((best_start..best_end).collect())
            } else {
                None
            };
            Some(MatchResult { score: best_score, matches })
        } else {
            None
        }
    }
}

fn build_failure_function(term: &[char]) -> Vec<isize> {
    let mut table: Vec<isize> = vec![-1; term.len() + 1];
        
    let mut pos = 1;
    let mut cnd = 0isize;

    table[0] = -1;

    while pos < term.len() {
        if term[pos] == term[cnd as usize] {
            table[pos] = table[cnd as usize];    
        } else {
            table[pos] = cnd as isize;
            cnd = table[cnd as usize];
            while cnd >= 0 && table[pos] != table[cnd as usize] {
                cnd = table[cnd as usize];
            }
        }
        pos += 1;
        cnd += 1;
    }
    table[pos] = cnd;
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_match_success {
        ($result: expr, $score: expr) => {{
            let result = $result;
            assert!(result.is_some(), stringify!($result));
            if let Some(result) = result {
                assert_eq!(result.score, $score, stringify!($result));
                assert_eq!(result.matches, None, stringify!($result));
            }
        }};
        ($result: expr, $score: expr, [ $($position: expr), + ]) => {{
            let result = $result;
            assert!(result.is_some(), stringify!($result));
            if let Some(result) = result {
                assert_eq!(result.score, $score, stringify!($result));
                assert_eq!(result.matches, Some(vec![$($position), +]), stringify!($result));
            }
        }};
    }

    macro_rules! assert_match_failure {
        ($result: expr) => {{
            assert!($result.is_none(), stringify!($result));
        }};
    }

    fn run_fuzzy_match(term: &str, input: &[u8], opts: MatchOptions) -> Option<MatchResult> {
        FuzzyMatcher::new(term).match_term(input, opts)
    }

    fn run_exact_match(term: &str, input: &[u8], opts: MatchOptions) -> Option<MatchResult> {
        ExactMatcher::new(term).match_term(input, opts)
    }

    static OPTS_DEFAULT: MatchOptions = MatchOptions { case_sensitive: false, match_position: false };
    static OPTS_POSITION: MatchOptions = MatchOptions { case_sensitive: false, match_position: true };

    #[test]
    fn fuzzy_matcher() {
        assert_match_success!(run_fuzzy_match("ABC", b"ADDD BDDD CDDD", OPTS_DEFAULT), SCORE_BEGINNING + 2 * SCORE_BOUNDARY);
        assert_match_success!(run_fuzzy_match("ABC", b"ABC", OPTS_DEFAULT),SCORE_BEGINNING + 2 * SCORE_MATCH + 2 * SCORE_CONSECUTIVE);
        assert_match_success!(run_fuzzy_match("ABC", b"DDD ADDD BDDD CDDD", OPTS_DEFAULT), 3 * SCORE_BOUNDARY);
        assert_match_success!(run_fuzzy_match("ABC", b"DDD ADDD BCDDD CDDD ABC", OPTS_DEFAULT), 2 * SCORE_BOUNDARY + SCORE_MATCH + SCORE_CONSECUTIVE);
        assert_match_success!(run_fuzzy_match("ABC", b"AB\xd8\x3fC", OPTS_DEFAULT), SCORE_BEGINNING + 2 * SCORE_MATCH + SCORE_CONSECUTIVE);
        
        assert_match_success!(run_fuzzy_match("ABC", b"ADDD BDDD CDDD", OPTS_POSITION), SCORE_BEGINNING + 2 * SCORE_BOUNDARY, [0,5,10]);
        assert_match_success!(run_fuzzy_match("ABC", b"ABC", OPTS_POSITION),SCORE_BEGINNING + 2 * SCORE_MATCH + 2 * SCORE_CONSECUTIVE, [0,1,2]);
        assert_match_success!(run_fuzzy_match("ABC", b"DDD ADDD BDDD CDDD", OPTS_POSITION), 3 * SCORE_BOUNDARY, [4,9,14]);
        assert_match_success!(run_fuzzy_match("ABC", b"DDD ADDD BCDDD CDDD ABC", OPTS_POSITION), 2 * SCORE_BOUNDARY + SCORE_MATCH + SCORE_CONSECUTIVE, [4,9,10]);
        assert_match_success!(run_fuzzy_match("ABC", b"AB\xd8\x3fC", OPTS_POSITION), SCORE_BEGINNING + 2 * SCORE_MATCH + SCORE_CONSECUTIVE, [0,1,4]);
        
        assert_match_failure!(run_fuzzy_match("ABC", b"AB", OPTS_DEFAULT));
        assert_match_failure!(run_fuzzy_match("ABC", b"DDD AB", OPTS_DEFAULT));
        assert_match_failure!(run_fuzzy_match("ABC", b"DDD ADDD BDDD", OPTS_DEFAULT));
    }

    #[test]
    fn exact_matcher() {   
        assert_match_success!(run_exact_match("ABC", b"ABC", OPTS_DEFAULT), SCORE_BEGINNING);
        assert_match_success!(run_exact_match("ABC", b"DDDABC", OPTS_DEFAULT), SCORE_MATCH);
        assert_match_success!(run_exact_match("ABC", b"DDDABC ABC", OPTS_DEFAULT), SCORE_BOUNDARY);
        
        assert_match_success!(run_exact_match("ABC", b"ABC", OPTS_POSITION), SCORE_BEGINNING, [0,1,2]);
        assert_match_success!(run_exact_match("ABC", b"DDDABC", OPTS_POSITION), SCORE_MATCH, [3,4,5]);
        assert_match_success!(run_exact_match("ABC", b"DDDABC ABC", OPTS_POSITION), SCORE_BOUNDARY, [7,8,9]);

        assert_match_failure!(run_exact_match("ABC", b"AB", OPTS_DEFAULT));
        assert_match_failure!(run_exact_match("ABC", b"AB\xd8\x3fC", OPTS_DEFAULT));
        assert_match_failure!(run_exact_match("ABC", b"ABDC", OPTS_DEFAULT));
    }
}