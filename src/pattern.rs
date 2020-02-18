use super::matcher::{Matcher, MatchResult, MatchOptions, FuzzyMatcher, ExactMatcher};

enum TermMatcher {
    Fuzzy(FuzzyMatcher),
    Exact(ExactMatcher),
}

impl Matcher for TermMatcher {
    fn match_term(&self, input: &[u8], opts: MatchOptions) -> Option<MatchResult> {
        match self {
            TermMatcher::Fuzzy(m) => m.match_term(input, opts), 
            TermMatcher::Exact(m) => m.match_term(input, opts), 
        }
    }
}

pub struct Pattern {
    terms: Vec<TermMatcher>,
    opts: MatchOptions,
}

impl Pattern {
    pub fn new(input: &str, opts: MatchOptions) -> Pattern {
        Pattern {
            opts,
            terms: parse_terms(input),
        }
    }

    pub fn matches(&self, input: &[u8]) -> Option<Vec<MatchResult>> {
        self.terms.iter().map(|m| m.match_term(input, self.opts)).collect()
    }
}

fn parse_term(term: &str) -> TermMatcher {
    match term.as_bytes().first().unwrap() {
        39 => TermMatcher::Exact(ExactMatcher::new(&term[1..term.len()])),
        _  => TermMatcher::Fuzzy(FuzzyMatcher::new(term)),
    }
}

fn parse_terms(input: &str) -> Vec<TermMatcher> {
    input.split_whitespace()
        .map(|t| parse_term(t))
        .collect()
}
