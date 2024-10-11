#![allow(dead_code, unused_imports, unused_variables)]

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::vec;

use crate::interegular::fsm::SymbolTrait;
use crate::interegular::fsm::{Alphabet, Fsm};
use crate::interegular::simple_parser::NoMatch;

const SPECIAL_CHARS_INNER: [&str; 2] = ["\\", "]"];
const SPECIAL_CHARS_STANDARD: [&str; 11] = ["+", "?", "*", ".", "$", "^", "\\", "(", ")", "[", "|"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegexElement {
    Literal(char),
    CharGroup {
        chars: BTreeSet<char>,
        inverted: bool,
    },
    Repeated {
        element: Box<RegexElement>,
        min: usize,
        max: Option<usize>,
    },
    Concatenation(Vec<RegexElement>),
    Alternation(Vec<RegexElement>),
    Capture(Box<RegexElement>),
    Group(Box<RegexElement>),
    Anchor(AnchorType),
    Flag {
        element: Box<RegexElement>,
        added: Vec<Flag>,
        removed: Vec<Flag>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnchorType {
    StartOfLine,
    EndOfLine,
    WordBoundary,
    NotWordBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Flag {
    CaseInsensitive,
    Multiline,
    DotMatchesNewline,
    Unicode,
}

impl From<usize> for Flag {
    fn from(value: usize) -> Self {
        match value {
            0 => Flag::CaseInsensitive,
            1 => Flag::Multiline,
            2 => Flag::DotMatchesNewline,
            3 => Flag::Unicode,
            _ => panic!("Invalid flag value"),
        }
    }
}

fn is_alphabetic(c: String) -> bool {
    c.chars().next().unwrap().is_alphabetic()
}

fn _combine_char_groups(groups: &[RegexElement], negate: bool) -> RegexElement {
    let mut pos = BTreeSet::new();
    let mut neg = BTreeSet::new();

    for group in groups {
        match group {
            RegexElement::CharGroup { chars, inverted } => {
                if *inverted {
                    neg.extend(chars.iter().copied());
                } else {
                    pos.extend(chars.iter().copied());
                }
            }
            _ => panic!("Invalid group type"),
        }
    }

    if !neg.is_empty() {
        RegexElement::CharGroup {
            chars: neg.difference(&pos).copied().collect(),
            inverted: !negate,
        }
    } else {
        RegexElement::CharGroup {
            chars: pos.difference(&neg).copied().collect(),
            inverted: negate,
        }
    }
}

impl RegexElement {
    #[must_use]
    pub fn repeat(self, min: usize, max: Option<usize>) -> Self {
        RegexElement::Repeated {
            element: Box::new(self),
            min,
            max,
        }
    }

    #[must_use]
    pub fn capture(self) -> Self {
        RegexElement::Capture(Box::new(self))
    }

    #[must_use]
    pub fn group(self) -> Self {
        RegexElement::Group(Box::new(self))
    }

    #[must_use]
    pub fn with_flags(self, added: Vec<Flag>, removed: Vec<Flag>) -> Self {
        RegexElement::Flag {
            element: Box::new(self),
            added,
            removed,
        }
    }
}

impl RegexElement {
    #[must_use]
    pub fn to_fsm(
        &self,
        alphabet: Option<Alphabet<char>>,
        prefix_postfix: Option<(usize, Option<usize>)>,
        flags: Option<BTreeSet<Flag>>,
    ) -> Fsm<char> {
        match self {
            RegexElement::Literal(c) => {
                let alphabet = alphabet
                    .unwrap_or_else(|| self.get_alphabet(&flags.clone().unwrap_or_default()));
                let prefix_postfix = prefix_postfix.unwrap_or_else(|| self.get_prefix_postfix());

                // let case_insensitive = flags
                //     .clone()
                //     .as_ref()
                //     .map_or(false, |f| f.contains(&Flag::CaseInsensitive));
                let case_insensitive = false;

                let mut mapping = BTreeMap::<_, BTreeMap<_, _>>::new();
                let symbol = alphabet.get(c);

                let mut m = std::collections::BTreeMap::new();
                m.insert(symbol, 1_usize);
                mapping.insert(0_usize, m);

                // states based on the symbols
                let unique_symbols = alphabet
                    .by_transition
                    .keys()
                    .copied()
                    .collect::<BTreeSet<_>>();

                let states = unique_symbols.iter().copied().collect();
                let finals = (1..=1).collect();

                Fsm::new(
                    alphabet, states, // {0, 1}
                    0, finals, // {1}
                    mapping,
                )
            }
            RegexElement::CharGroup { chars, inverted } => {
                let alphabet = alphabet
                    .unwrap_or_else(|| self.get_alphabet(&flags.clone().unwrap_or_default()));
                let prefix_postfix = prefix_postfix.unwrap_or_else(|| self.get_prefix_postfix());

                assert!(
                    prefix_postfix == (0, Some(0)),
                    "Cannot have prefix/postfix on CharGroup-level"
                );

                // let case_insensitive = flags
                //     .clone()
                //     .as_ref()
                //     .map_or(false, |f| f.contains(&Flag::CaseInsensitive));
                let case_insensitive = false;

                let mut mapping = BTreeMap::<_, BTreeMap<_, _>>::new();

                if *inverted {
                    let chars = chars.clone();
                    let alphabet = alphabet.clone();
                    let alphabet_set = alphabet
                        .clone()
                        .by_transition
                        .keys()
                        .copied()
                        .collect::<BTreeSet<_>>();

                    let char_as_usize = chars.iter().map(|c| *c as usize).collect();
                    let diff = alphabet_set
                        .difference(&char_as_usize)
                        .copied()
                        .collect::<Vec<_>>();

                    let mut m = std::collections::BTreeMap::new();
                    for symbol in diff {
                        m.insert(symbol, 1_usize);
                    }
                    mapping.insert(0_usize, m);
                } else {
                    let chars = chars.clone();
                    for symbol in chars {
                        let mut m = std::collections::BTreeMap::new();
                        let symbol_value = alphabet.get(&symbol);
                        m.insert(symbol_value, 1_usize);
                        mapping.insert(0_usize, m);
                    }
                }

                let states = (0..=1).collect();
                let finals = (1..=1).collect();

                Fsm::new(
                    alphabet, states, // {0, 1}
                    0, finals, // {1}
                    mapping,
                )
            }
            RegexElement::Repeated { element, min, max } => {
                // # REF
                // def to_fsm(self, alphabet=None, prefix_postfix=None, flags=REFlags(0)) -> FSM:
                //     if alphabet is None:
                //         alphabet = self.get_alphabet(flags)
                //     if prefix_postfix is None:
                //         prefix_postfix = self.prefix_postfix
                //     if prefix_postfix != (0, 0):
                //         raise ValueError("Can not have prefix/postfix on CharGroup-level")
                //     print("alphabet", alphabet.__dict__)
                //     unit = self.base.to_fsm(alphabet, (0, 0), flags=flags)
                //     print("unit", unit.__dict__)
                //     mandatory = unit * self.min
                //     print("mandatory", mandatory.__dict__, self.min)
                //     if self.max is None:
                //         optional = unit.star()
                //     else:
                //         optional = unit.copy()
                //         optional.__dict__['finals'] |= {optional.initial}
                //         optional *= (self.max - self.min)
                //     return mandatory + optional

                let unit = element.to_fsm(alphabet.clone(), None, flags.clone());
                let alphabet = alphabet
                    .unwrap_or_else(|| self.get_alphabet(&flags.clone().unwrap_or_default()));

                let base_fsm = element.to_fsm(Some(alphabet.clone()), None, flags.clone());
                let mandatory = std::iter::repeat(base_fsm.clone()).take(*min).fold(
                    Fsm::new(
                        alphabet.clone(),
                        BTreeSet::from([0]),
                        0,
                        BTreeSet::from([0]),
                        std::collections::BTreeMap::from([(0, BTreeMap::new())]),
                    ),
                    |acc, f| Fsm::concatenate(&[acc, f]),
                );

                let optional = if max.is_none() {
                    unit.star()
                } else {
                    let mut optional = unit.clone();
                    optional.finals.insert(optional.initial);
                    optional = std::iter::repeat(optional.clone())
                        .take(max.unwrap() - min)
                        .fold(
                            Fsm::new(
                                alphabet.clone(),
                                BTreeSet::new(),
                                0,
                                BTreeSet::new(),
                                std::collections::BTreeMap::new(),
                            ),
                            |acc, f| Fsm::concatenate(&[acc, f]),
                        );

                    optional
                };

                Fsm::concatenate(&[mandatory, optional])
            }
            RegexElement::Concatenation(parts) => {
                let mut current = vec![];
                for part in parts {
                    current.push(part.to_fsm(alphabet.clone(), None, flags.clone()));
                }

                Fsm::concatenate(&current)
            }
            RegexElement::Alternation(options) => {
                let mut current = vec![];
                for option in options {
                    current.push(option.to_fsm(alphabet.clone(), None, flags.clone()));
                }

                Fsm::union(&current)
            }
            // throw on non implemented variants
            _ => unimplemented!("FSM conversion not implemented for this variant"),
        }
    }

    pub fn get_alphabet<T: SymbolTrait + std::cmp::Ord>(
        &self,
        flags: &BTreeSet<Flag>,
    ) -> Alphabet<T> {
        match self {
            RegexElement::CharGroup { chars, .. } => {
                // let case_insensitive = flags.contains(&Flag::CaseInsensitive);
                let case_insensitive = false;
                let relevant = if case_insensitive {
                    chars
                        .iter()
                        // .flat_map(|c| vec![c.to_ascii_lowercase(), c.to_ascii_uppercase()])
                        .flat_map(|c| vec![])
                        .collect()
                } else {
                    chars.iter().map(|c| (*c).into()).collect()
                };
                // Alphabet::from_groups(&[relevant, BTreeSet::from([TransitionKey::AnythingElse])])
                Alphabet::from_groups(&[relevant, BTreeSet::from(['\0'.into()])])
            }
            RegexElement::Literal(c) => Alphabet::from_groups(&[BTreeSet::from([(*c).into()])]),
            RegexElement::Repeated { element, .. } => element.get_alphabet(flags),
            RegexElement::Alternation(options) => {
                let mut alphabet = Alphabet::empty();
                for option in options {
                    let alphabets = vec![alphabet, option.get_alphabet(flags)];
                    let (res, new_to_old) = Alphabet::union(alphabets.as_slice());
                    alphabet = res;
                }
                alphabet
            }
            RegexElement::Concatenation(parts) => {
                let mut alphabet = Alphabet::empty();
                for part in parts {
                    let alphabets = vec![alphabet, part.get_alphabet(flags)];
                    let (res, new_to_old) = Alphabet::union(alphabets.as_slice());
                    alphabet = res;
                }
                alphabet
            }
            _ => unimplemented!("Alphabet not implemented for this variant"),
        }
    }

    #[must_use]
    pub fn get_prefix_postfix(&self) -> (usize, Option<usize>) {
        match self {
            RegexElement::CharGroup { .. } => (0, Some(0)),
            RegexElement::Literal(_) => (1, Some(1)),
            RegexElement::Repeated { element, min, max } => {
                let (l, h) = element.get_prefix_postfix();
                (l * min, max.and_then(|max| h.map(|h| h * max)))
            }
            RegexElement::Concatenation(parts) => {
                let mut pre = 0;
                let mut post = Some(0);

                for part in parts {
                    let (o_pre, o_post) = part.get_prefix_postfix();
                    pre = pre.max(o_pre);
                    post = match (post, o_post) {
                        (Some(p), Some(o)) => Some(p.max(o)),
                        (None, o) => o,
                        (p, None) => p,
                    };
                }

                (pre, post)
            }
            RegexElement::Alternation(options) => {
                let mut pre = 0;
                let mut post = Some(0);

                for option in options {
                    let (o_pre, o_post) = option.get_prefix_postfix();
                    pre = pre.max(o_pre);
                    post = match (post, o_post) {
                        (Some(p), Some(o)) => Some(p.max(o)),
                        (None, o) => o,
                        (p, None) => p,
                    };
                }

                (pre, post)
            }
            RegexElement::Capture(inner) => inner.get_prefix_postfix(),
            RegexElement::Group(inner) => inner.get_prefix_postfix(),
            RegexElement::Anchor(_) => (0, Some(0)),
            RegexElement::Flag { element, .. } => element.get_prefix_postfix(),
        }
    }

    #[must_use]
    pub fn get_lengths(&self) -> (usize, Option<usize>) {
        match self {
            RegexElement::CharGroup { .. } => (1, Some(1)),
            RegexElement::Literal(_) => (1, Some(1)),
            RegexElement::Repeated { element, min, max } => {
                let (l, h) = element.get_lengths();
                (l * min, max.and_then(|max| h.map(|h| h * max)))
            }
            RegexElement::Concatenation(parts) => {
                let mut low = 0;
                let mut high = Some(0);

                for part in parts {
                    let (l, h) = part.get_lengths();
                    low += l;
                    high = high.and_then(|high| h.map(|h| high + h));
                }

                (low, high)
            }
            RegexElement::Alternation(options) => {
                let mut low = None;
                let mut high = Some(0);

                for option in options {
                    let (l, h) = option.get_lengths();
                    low = Some(low.map_or(l, |low: usize| low.min(l)));
                    high = match (high, h) {
                        (Some(high), Some(h)) => Some(high.max(h)),
                        _ => None,
                    };
                }

                (low.unwrap_or(0), high)
            }
            RegexElement::Capture(inner) => inner.get_lengths(),
            RegexElement::Group(inner) => inner.get_lengths(),
            RegexElement::Anchor(_) => (0, Some(0)),
            RegexElement::Flag { element, .. } => element.get_lengths(),
        }
    }

    pub fn simplify(&self) -> RegexElement {
        match self {
            RegexElement::Alternation(options) => {
                if options.len() == 1 {
                    let o = &options[0];
                    if let RegexElement::Concatenation(parts) = o {
                        // must be len 1 and an alternation
                        if parts.len() == 1 {
                            if let RegexElement::Alternation(_options) = &parts[0] {
                                return parts[0].simplify();
                            }
                        }
                    }
                }
                let mut new_options = vec![];
                for option in options {
                    new_options.push(option.simplify());
                }
                RegexElement::Alternation(new_options)
            }
            RegexElement::Repeated { element, min, max } => RegexElement::Repeated {
                element: Box::new(element.simplify()),
                min: *min,
                max: max.clone(),
            },
            RegexElement::Concatenation(parts) => {
                let mut new_parts = vec![];
                for part in parts {
                    new_parts.push(part.simplify());
                }
                self.clone()
            }
            _ => self.clone(),
        }
    }

    #[must_use]
    pub fn to_concrete(&self) -> RegexElement {
        self.clone()
    }
}

pub struct ParsePattern<'a> {
    parser: crate::interegular::simple_parser::SimpleParser<RegexElement>,
    flags: Option<Vec<usize>>,
    data: &'a str,
}

impl<'a> ParsePattern<'a> {
    #[must_use]
    pub fn new(data: &'a str) -> Self {
        ParsePattern {
            parser: crate::interegular::simple_parser::SimpleParser::new(data),
            flags: None,
            data,
        }
    }

    pub fn parse(&mut self) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        let result = self.start()?;
        if self.parser.index < self.data.len() {
            let max_index = *self.parser.expected.keys().max().unwrap_or(&0);
            Err(crate::interegular::simple_parser::NoMatch::new(
                self.data,
                max_index,
                self.parser
                    .expected
                    .get(&max_index)
                    .unwrap_or(&vec![])
                    .clone(),
            ))
        } else {
            Ok(result)
        }
    }

    fn start(&mut self) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        self.flags = None;
        let p = self.pattern()?;
        if let Some(flags) = self.flags.take() {
            Ok(p.with_flags(flags.iter().map(|f| Flag::from(*f)).collect(), vec![]))
        } else {
            Ok(p)
        }
    }

    fn pattern(&mut self) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        let mut options = vec![self.conc()?];
        while self.parser.static_b("|") {
            options.push(self.conc()?);
        }
        Ok(RegexElement::Alternation(options))
    }

    fn conc(&mut self) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        let mut parts = vec![];
        while let Ok(obj) = self.obj() {
            parts.push(obj);
        }
        Ok(RegexElement::Concatenation(parts))
    }

    fn obj(&mut self) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        if self.parser.static_b("(") {
            self.group()
        } else {
            match self.atom() {
                Ok(atom) => self.repetition(atom),
                Err(_) => Err(crate::interegular::simple_parser::NoMatch::new(
                    self.data,
                    self.parser.index,
                    vec!["(".to_string()],
                )),
            }
        }
    }

    fn atom(&mut self) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        if self.parser.static_b("[") {
            match self.chargroup() {
                Ok(cg) => self.repetition(cg),
                Err(_) => Err(crate::interegular::simple_parser::NoMatch::new(
                    self.data,
                    self.parser.index,
                    vec!["[".to_string()],
                )),
            }
        } else if self.parser.static_b("\\") {
            match self.escaped(false) {
                Ok(cg) => self.repetition(cg),
                Err(_) => Err(crate::interegular::simple_parser::NoMatch::new(
                    self.data,
                    self.parser.index,
                    vec!["\\".to_string()],
                )),
            }
        } else if self.parser.static_b(".") {
            let cg = RegexElement::CharGroup {
                chars: vec!['\n'].into_iter().collect(),
                inverted: true,
            };
            self.repetition(cg)
        } else if self.parser.static_b("$") {
            // Unsupported
            Err(crate::interegular::simple_parser::NoMatch::new(
                self.data,
                self.parser.index,
                vec!["'$'".to_string()],
            ))
        } else if self.parser.static_b("^") {
            // Unsupported
            Err(crate::interegular::simple_parser::NoMatch::new(
                self.data,
                self.parser.index,
                vec!["'^'".to_string()],
            ))
        } else {
            let c = self.parser.any_but(&SPECIAL_CHARS_STANDARD, 1)?;
            let cg = RegexElement::CharGroup {
                chars: vec![c.chars().next().unwrap()].into_iter().collect(),
                inverted: false,
            };
            self.repetition(cg)
        }
    }

    fn group(&mut self) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        if self.parser.static_b("?") {
            self.extension_group()
        } else {
            let p = self.pattern().unwrap();
            self.parser.static_b(")");
            self.repetition(p)
        }
    }

    fn extension_group(
        &mut self,
    ) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        let c = self.parser.any(1)?;
        if "aiLmsux-".contains(&c) {
            self.parser.index -= 1;
            let added_flags = self.parser.multiple("aiLmsux", 0, None)?;
            let removed_flags = if self.parser.static_b("-") {
                self.parser.multiple("aiLmsux", 1, None)?
            } else {
                String::new()
            };

            // TODO: missing cases
        }
        unimplemented!("Missing cases")
    }

    fn repetition(
        &mut self,
        base: RegexElement,
    ) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        if self.parser.static_b("*") {
            self.parser.static_b("?");
            Ok(RegexElement::Repeated {
                element: Box::new(base),
                min: 0,
                max: None,
            })
        } else if self.parser.static_b("+") {
            self.parser.static_b("?");
            Ok(RegexElement::Repeated {
                element: Box::new(base),
                min: 1,
                max: None,
            })
        } else if self.parser.static_b("?") {
            self.parser.static_b("?");
            Ok(RegexElement::Repeated {
                element: Box::new(base),
                min: 0,
                max: Some(1),
            })
        } else if self.parser.static_b("{") {
            let n = self.number().unwrap_or(0);
            let m = if self.parser.static_b(",") {
                match self.number() {
                    Ok(num) => Some(num),
                    Err(_) => None,
                }
            } else {
                Some(n)
            };
            let _ = self.parser.static_match("}");
            self.parser.static_b("?");
            Ok(RegexElement::Repeated {
                element: Box::new(base),
                min: n,
                max: m,
            })
        } else {
            Ok(base)
        }
    }

    fn number(&mut self) -> Result<usize, crate::interegular::simple_parser::NoMatch> {
        let num = self.parser.multiple("0123456789", 1, None)?;
        Ok(num.parse().unwrap())
    }

    fn chargroup(&mut self) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        let negate = self.parser.static_b("^");
        let mut groups = vec![];
        while let Ok(group) = self.chargroup_inner() {
            groups.push(group);
        }
        let _ = self.parser.static_match("]");
        if groups.len() == 1 {
            let f = groups[0].clone();
            match f {
                RegexElement::CharGroup { chars, inverted } => Ok(RegexElement::CharGroup {
                    chars,
                    inverted: inverted ^ negate,
                }),
                _ => panic!("Invalid group type"),
            }
        } else if groups.is_empty() {
            Ok(RegexElement::CharGroup {
                chars: BTreeSet::new(),
                inverted: negate,
            })
        } else {
            Ok(_combine_char_groups(&groups, negate))
        }
    }

    fn chargroup_inner(
        &mut self,
    ) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        let base = if self.parser.static_b("\\") {
            self.escaped(true)
        } else {
            let c = self.parser.any_but(&SPECIAL_CHARS_INNER, 1)?;
            Ok(RegexElement::CharGroup {
                chars: vec![c.chars().next().unwrap()].into_iter().collect(),
                inverted: false,
            })
        };
        let base_copy = base.clone();
        if self.parser.static_b("-") {
            let end = if self.parser.static_b("\\") {
                self.escaped(true)?
            } else if self.parser.peek_static("]") {
                // this case we have `X-]` which needs to include the `-` in
                // the group since it's not a range
                return Ok(_combine_char_groups(
                    &[
                        base_copy?,
                        RegexElement::CharGroup {
                            chars: vec!['-'].into_iter().collect(),
                            inverted: false,
                        },
                    ],
                    false,
                ));
            } else {
                let c = self.parser.any_but(&SPECIAL_CHARS_INNER, 1)?;
                RegexElement::CharGroup {
                    chars: vec![c.chars().next().unwrap()].into_iter().collect(),
                    inverted: false,
                }
            };

            let low = match base? {
                RegexElement::CharGroup { chars, .. } => *chars.iter().next().unwrap(),
                _ => panic!("Invalid group type"),
            };
            let high = match end {
                RegexElement::CharGroup { chars, .. } => *chars.iter().next().unwrap(),
                _ => panic!("Invalid group type"),
            };

            assert!(low <= high, "Invalid Character-range");

            let chars = (low..=high).collect();
            return Ok(RegexElement::CharGroup {
                chars,
                inverted: false,
            });
        }

        base
    }

    fn escaped(
        &mut self,
        inner: bool,
    ) -> Result<RegexElement, crate::interegular::simple_parser::NoMatch> {
        if self.parser.static_b("x") {
            let n = self.parser.multiple("0123456789abcdefABCDEF", 2, Some(2))?;
            let c = char::from_u32(u32::from_str_radix(&n, 16).unwrap()).unwrap();
            return Ok(RegexElement::CharGroup {
                chars: vec![c].into_iter().collect(),
                inverted: false,
            });
        } else if self.parser.static_b("0") {
            let n = self.parser.multiple("01234567", 1, Some(2))?;
            let c = char::from_u32(u32::from_str_radix(&n, 8).unwrap()).unwrap();
            return Ok(RegexElement::CharGroup {
                chars: vec![c].into_iter().collect(),
                inverted: false,
            });
        } else if self.parser.anyof_b(&["N", "p", "P", "u", "U"]) {
            unimplemented!("regex module unicode properties are not supported.")
        }

        if !inner {
            let n = self
                .parser
                .multiple("01234567", 3, Some(3))
                .unwrap_or_default();
            if !n.is_empty() {
                let c = char::from_u32(u32::from_str_radix(&n, 8).unwrap()).unwrap();
                return Ok(RegexElement::CharGroup {
                    chars: vec![c].into_iter().collect(),
                    inverted: false,
                });
            } else {
                let n = self
                    .parser
                    .multiple("0123456789", 1, Some(2))
                    .unwrap_or_default();
                if !n.is_empty() {
                    unimplemented!("Group references are not implemented")
                } else {
                    let n = self.parser.any_but(
                        &[
                            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n",
                            "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z", "A", "B",
                            "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P",
                            "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
                        ],
                        1,
                    )?;
                    let c = n.chars().next().unwrap();
                    if c.is_alphabetic() {
                        return Err(crate::interegular::simple_parser::NoMatch::new(
                            self.data,
                            self.parser.index,
                            vec![n],
                        ));
                    } else {
                        return Ok(RegexElement::CharGroup {
                            chars: vec![c].into_iter().collect(),
                            inverted: false,
                        });
                    }
                }
            }
        }

        // this is effectively the else branch of the if !inner check
        let n = self.parser.multiple("01234567", 1, Some(3));
        match n {
            Ok(n) => {
                let c = char::from_u32(u32::from_str_radix(&n, 8).unwrap()).unwrap();
                Ok(RegexElement::CharGroup {
                    chars: vec![c].into_iter().collect(),
                    inverted: false,
                })
            }
            Err(_) => {
                let c = self.parser.anyof(&[
                    "w", "W", "d", "D", "s", "S", "a", "b", "f", "n", "r", "t", "v",
                ]);

                match c {
                    Ok(c) => {
                        let chars = match c.as_str() {
                            "w" => vec![
                                'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
                                'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
                                'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
                                'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
                                '_', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
                            ],
                            "W" => vec!['\n', '\r', '\t', '\x0b', '\x0c', ' '],
                            "d" => vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
                            "D" => vec!['\n', '\r', '\t', '\x0b', '\x0c', ' '],
                            "s" => vec!['\n', '\r', '\t', '\x0b', '\x0c', ' '],
                            "S" => vec!['\n', '\r', '\t', '\x0b', '\x0c', ' '],
                            "a" => vec!['\x07'],
                            "b" => vec!['\x08'],
                            "f" => vec!['\x0c'],
                            "n" => vec!['\n'],
                            "r" => vec!['\r'],
                            "t" => vec!['\t'],
                            "v" => vec!['\x0b'],
                            _ => panic!("Invalid escape character"),
                        };
                        Ok(RegexElement::CharGroup {
                            chars: chars.into_iter().collect(),
                            inverted: false,
                        })
                    }
                    Err(_) => {
                        let c = self.parser.any_but(
                            &[
                                "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
                                "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
                                "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M",
                                "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
                            ],
                            1,
                        )?;
                        if is_alphabetic(c.clone()) {
                            Err(crate::interegular::simple_parser::NoMatch::new(
                                self.data,
                                self.parser.index,
                                vec![c],
                            ))
                        } else {
                            Ok(RegexElement::CharGroup {
                                chars: c.chars().collect(),
                                inverted: false,
                            })
                        }
                    }
                }
            }
        }
    }
}

pub fn parse_pattern(pattern: &str) -> Result<RegexElement, super::simple_parser::NoMatch> {
    let mut parser = ParsePattern::new(pattern);
    match parser.parse() {
        Ok(raw_result) => Ok(raw_result.simplify()),
        Err(e) => Err(e),
    }
}

pub fn parse_pattern_to_fms(pattern: &str) -> Fsm<char> {
    let regex_element = parse_pattern(pattern).unwrap();

    let prefix_postfix = None;
    let flags = None;

    let default_alphabet = Alphabet::<char>::default();
    let empty_flags: BTreeSet<Flag> = BTreeSet::new();
    let patterns_alphabet: Alphabet<char> = regex_element.get_alphabet(&empty_flags);

    let mut new_symbol_mapping: BTreeMap<char, usize> = BTreeMap::new();
    let mut new_by_transition: BTreeMap<usize, Vec<char>> = BTreeMap::new();
    new_symbol_mapping.insert('\0', 0);
    for (symbol, index) in patterns_alphabet.symbol_mapping.iter() {
        if *symbol != '\0' {
            let new_index = index + 1;
            new_symbol_mapping.insert(*symbol, new_index);
            // add to the existing transitions if it exists
            if new_by_transition.contains_key(&new_index) {
                let transitions = new_by_transition.get_mut(&new_index).unwrap();
                transitions.push(*symbol);
            } else {
                new_by_transition.insert(new_index, vec![*symbol]);
            }
        }
    }
    let alphabet = Alphabet {
        symbol_mapping: new_symbol_mapping,
        by_transition: new_by_transition,
    };
    let fsm_info = regex_element.to_fsm(Some(alphabet.clone()), prefix_postfix, flags);

    fsm_info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pattern_simple() {
        let pattern: &str = "a";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                    chars: vec!['a'].into_iter().collect(),
                    inverted: false
                }])
            ]))
        );
    }

    #[test]
    fn test_parse_pattern_alternation() {
        let pattern = "a|b|c";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                    chars: vec!['a'].into_iter().collect(),
                    inverted: false
                }]),
                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                    chars: vec!['b'].into_iter().collect(),
                    inverted: false
                }]),
                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                    chars: vec!['c'].into_iter().collect(),
                    inverted: false
                }])
            ]))
        );
    }

    #[test]
    fn test_parse_pattern_concatenation() {
        let pattern = "abc";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![
                    RegexElement::CharGroup {
                        chars: vec!['a'].into_iter().collect(),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: vec!['b'].into_iter().collect(),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: vec!['c'].into_iter().collect(),
                        inverted: false
                    }
                ])
            ]))
        );
    }

    #[test]
    fn test_parse_pattern_repetition() {
        let pattern = "a*b+c?";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![
                    RegexElement::Repeated {
                        element: Box::new(RegexElement::CharGroup {
                            chars: vec!['a'].into_iter().collect(),
                            inverted: false
                        }),
                        min: 0,
                        max: None
                    },
                    RegexElement::Repeated {
                        element: Box::new(RegexElement::CharGroup {
                            chars: vec!['b'].into_iter().collect(),
                            inverted: false
                        }),
                        min: 1,
                        max: None
                    },
                    RegexElement::Repeated {
                        element: Box::new(RegexElement::CharGroup {
                            chars: vec!['c'].into_iter().collect(),
                            inverted: false
                        }),
                        min: 0,
                        max: Some(1)
                    }
                ])
            ]))
        );
    }

    #[test]
    fn test_parse_pattern_chargroup() {
        let pattern = "[abc]";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                    chars: vec!['a', 'b', 'c'].into_iter().collect(),
                    inverted: false
                }])
            ]))
        );
    }

    #[test]
    fn test_parse_pattern_negated_chargroup() {
        let pattern = "[^abc]";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                    chars: vec!['a', 'b', 'c'].into_iter().collect(),
                    inverted: true
                }])
            ]))
        );
    }

    #[test]
    fn test_parse_pattern_escaped_chars() {
        let pattern = r"\.\*\+\?\|\(\)\[\]\{\}\^\$";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['.']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['*']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['+']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['?']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['|']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['(']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from([')']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['[']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from([']']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['{']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['}']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['^']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['$']),
                        inverted: false
                    }
                ]),
            ]))
        );
    }

    // #[test]
    // fn test_parse_pattern_complex() {
    //     let pattern = r"(a|b)*c+[def]\d{2,4}";
    //     let result = parse_pattern(pattern);
    //     assert_eq!(
    //         result,
    //         Ok(RegexElement::Alternation(vec![
    //             RegexElement::Concatenation(vec![
    //                 RegexElement::Repeated {
    //                     element: Box::new(RegexElement::Group(Box::new(
    //                         RegexElement::Alternation(vec![
    //                             RegexElement::Concatenation(vec![RegexElement::Literal('a')]),
    //                             RegexElement::Concatenation(vec![RegexElement::Literal('b')])
    //                         ])
    //                     ))),
    //                     min: 0,
    //                     max: None
    //                 },
    //                 RegexElement::Repeated {
    //                     element: Box::new(RegexElement::Literal('c')),
    //                     min: 1,
    //                     max: None
    //                 },
    //                 RegexElement::CharGroup {
    //                     chars: vec!['d', 'e', 'f'].into_iter().collect(),
    //                     inverted: false
    //                 },
    //                 RegexElement::Repeated {
    //                     element: Box::new(RegexElement::CharGroup {
    //                         chars: ('0'..='9').collect(),
    //                         inverted: false
    //                     }),
    //                     min: 2,
    //                     max: Some(4)
    //                 }
    //             ])
    //         ]))
    //     );
    // }

    #[test]
    fn test_parse_pattern_dot() {
        let pattern = "a.b";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![
                    RegexElement::CharGroup {
                        chars: vec!['a'].into_iter().collect(),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: vec!['\n'].into_iter().collect(),
                        inverted: true
                    },
                    RegexElement::CharGroup {
                        chars: vec!['b'].into_iter().collect(),
                        inverted: false
                    }
                ])
            ]))
        );
    }

    #[test]
    fn test_parse_range() {
        let pattern = "[a-f]";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                    chars: ('a'..='f').collect(),
                    inverted: false
                }])
            ]))
        );
    }

    #[test]
    fn test_parse_pattern_repeat() {
        let pattern = "a{3,6}";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![RegexElement::Repeated {
                    element: Box::new(RegexElement::CharGroup {
                        chars: vec!['a'].into_iter().collect(),
                        inverted: false
                    }),
                    min: 3,
                    max: Some(6)
                }])
            ]))
        );
    }

    #[test]
    fn test_parse_pattern_anchors() {
        let pattern = "abc$";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['a']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['b']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['c']),
                        inverted: false
                    },
                ])
            ]))
        );
    }

    #[test]
    fn test_parse_pattern_invalid() {
        let pattern = ")(";
        let result = parse_pattern(pattern);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pattern_string_pattern() {
        let pattern = "\"([^\"\\\\\\x00-\\x1F\\x7F-\\x9F]|\\\\[\"\\\\])*\"";
        let result = parse_pattern(&pattern);
        let ascii_chars: BTreeSet<u8> = (0x00..=0x1F).chain(0x7F..=0x9F).collect();
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['"',]),
                        inverted: false,
                    },
                    RegexElement::Repeated {
                        element: Box::new(RegexElement::Alternation(vec![
                            RegexElement::Concatenation(vec![RegexElement::CharGroup {
                                chars: BTreeSet::from([
                                    '\0', '\u{1}', '\u{2}', '\u{3}', '\u{4}', '\u{5}', '\u{6}',
                                    '\u{7}', '\u{8}', '\t', '\n', '\u{b}', '\u{c}', '\r', '\u{e}',
                                    '\u{f}', '\u{10}', '\u{11}', '\u{12}', '\u{13}', '\u{14}',
                                    '\u{15}', '\u{16}', '\u{17}', '\u{18}', '\u{19}', '\u{1a}',
                                    '\u{1b}', '\u{1c}', '\u{1d}', '\u{1e}', '\u{1f}', '"', '\\',
                                    '\u{7f}', '\u{80}', '\u{81}', '\u{82}', '\u{83}', '\u{84}',
                                    '\u{85}', '\u{86}', '\u{87}', '\u{88}', '\u{89}', '\u{8a}',
                                    '\u{8b}', '\u{8c}', '\u{8d}', '\u{8e}', '\u{8f}', '\u{90}',
                                    '\u{91}', '\u{92}', '\u{93}', '\u{94}', '\u{95}', '\u{96}',
                                    '\u{97}', '\u{98}', '\u{99}', '\u{9a}', '\u{9b}', '\u{9c}',
                                    '\u{9d}', '\u{9e}', '\u{9f}',
                                ]),
                                inverted: true,
                            },],),
                            RegexElement::Concatenation(vec![
                                RegexElement::CharGroup {
                                    chars: BTreeSet::from(['\\',]),
                                    inverted: false,
                                },
                                RegexElement::CharGroup {
                                    chars: BTreeSet::from(['"', '\\',]),
                                    inverted: false,
                                },
                            ],),
                        ],)),
                        min: 0,
                        max: None,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['"',]),
                        inverted: false,
                    },
                ],),
            ]))
        );
    }

    #[test]
    fn test_parse_pattern_enum_string() {
        let pattern = "(\"Marc\"|\"Jean\")";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['"']),
                        inverted: false,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['M']),
                        inverted: false,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['a']),
                        inverted: false,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['r']),
                        inverted: false,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['c']),
                        inverted: false,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['"']),
                        inverted: false,
                    },
                ]),
                RegexElement::Concatenation(vec![
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['"']),
                        inverted: false,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['J']),
                        inverted: false,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['e']),
                        inverted: false,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['a']),
                        inverted: false,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['n']),
                        inverted: false,
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['"']),
                        inverted: false,
                    },
                ]),
            ]))
        )
    }

    #[test]
    fn test_parse_pattern_enum_char() {
        let pattern = "(A|B)";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                    chars: BTreeSet::from(['A']),
                    inverted: false
                }]),
                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                    chars: BTreeSet::from(['B']),
                    inverted: false
                }]),
            ]))
        )
    }

    #[test]
    fn test_parse_pattern_null() {
        let pattern = "null";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['n']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['u']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['l']),
                        inverted: false
                    },
                    RegexElement::CharGroup {
                        chars: BTreeSet::from(['l']),
                        inverted: false
                    },
                ]),
            ]))
        )
    }

    #[test]
    fn test_parse_pattern_number() {
        let pattern = "((-)?(0|[1-9][0-9]*))(\\.[0-9]+)?([eE][+-][0-9]+)?";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![
                    RegexElement::Alternation(vec![RegexElement::Concatenation(vec![
                        RegexElement::Repeated {
                            element: Box::new(RegexElement::Alternation(vec![
                                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                                    chars: BTreeSet::from(['-',]),
                                    inverted: false,
                                },]),
                            ])),
                            min: 0,
                            max: Some(1),
                        },
                        RegexElement::Alternation(vec![
                            RegexElement::Concatenation(vec![RegexElement::CharGroup {
                                chars: BTreeSet::from(['0',]),
                                inverted: false,
                            }]),
                            RegexElement::Concatenation(vec![
                                RegexElement::CharGroup {
                                    chars: BTreeSet::from([
                                        '1', '2', '3', '4', '5', '6', '7', '8', '9',
                                    ]),
                                    inverted: false,
                                },
                                RegexElement::Repeated {
                                    element: Box::new(RegexElement::CharGroup {
                                        chars: BTreeSet::from([
                                            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
                                        ]),
                                        inverted: false,
                                    }),
                                    min: 0,
                                    max: None,
                                },
                            ]),
                        ]),
                    ]),]),
                    RegexElement::Repeated {
                        element: Box::new(RegexElement::Alternation(vec![
                            RegexElement::Concatenation(vec![
                                RegexElement::CharGroup {
                                    chars: BTreeSet::from(['.',]),
                                    inverted: false,
                                },
                                RegexElement::Repeated {
                                    element: Box::new(RegexElement::CharGroup {
                                        chars: BTreeSet::from([
                                            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
                                        ]),
                                        inverted: false,
                                    }),
                                    min: 1,
                                    max: None,
                                },
                            ]),
                        ])),
                        min: 0,
                        max: Some(1),
                    },
                    RegexElement::Repeated {
                        element: Box::new(RegexElement::Alternation(vec![
                            RegexElement::Concatenation(vec![
                                RegexElement::CharGroup {
                                    chars: BTreeSet::from(['E', 'e',]),
                                    inverted: false,
                                },
                                RegexElement::CharGroup {
                                    chars: BTreeSet::from(['+', '-',]),
                                    inverted: false,
                                },
                                RegexElement::Repeated {
                                    element: Box::new(RegexElement::CharGroup {
                                        chars: BTreeSet::from([
                                            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
                                        ]),
                                        inverted: false,
                                    }),
                                    min: 1,
                                    max: None,
                                },
                            ]),
                        ])),
                        min: 0,
                        max: Some(1),
                    },
                ]),
            ]))
        )
    }

    #[test]
    fn test_parse_pattern_literal_digit() {
        let pattern = "0";
        let result = parse_pattern(pattern);
        assert_eq!(
            result,
            Ok(RegexElement::Alternation(vec![
                RegexElement::Concatenation(vec![RegexElement::CharGroup {
                    chars: BTreeSet::from(['0']),
                    inverted: false
                }]),
            ]))
        )
    }

    #[test]
    fn test_parse_pattern_simple_to_fsm() {
        let pattern: &str = "a";
        let result = parse_pattern(pattern).unwrap();

        let alphabet = Alphabet {
            symbol_mapping: BTreeMap::from([('a', 1), ('\0', 0)]),
            by_transition: BTreeMap::from([(0, vec!['\0']), (1, vec!['a'])]),
        };

        let result = result.to_fsm(Some(alphabet.clone()), None, None);

        let expected = Fsm {
            alphabet,
            states: BTreeSet::from([0, 1]),
            initial: 0,
            finals: BTreeSet::from([1]),
            map: BTreeMap::from([(0, BTreeMap::from([(1, 1)])), (1, BTreeMap::new())]),
        };

        assert_eq!(
            result
                .alphabet
                .symbol_mapping
                .keys()
                .copied()
                .collect::<BTreeSet<_>>(),
            expected
                .alphabet
                .symbol_mapping
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
        );

        assert_eq!(
            result
                .alphabet
                .by_transition
                .keys()
                .copied()
                .collect::<BTreeSet<_>>(),
            expected
                .alphabet
                .by_transition
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
        );

        assert_eq!(
            result.states.iter().copied().collect::<BTreeSet<_>>(),
            expected.states.iter().copied().collect::<BTreeSet<_>>()
        );

        assert_eq!(result.initial, expected.initial);

        assert_eq!(
            result.finals.iter().copied().collect::<BTreeSet<_>>(),
            expected.finals.iter().copied().collect::<BTreeSet<_>>()
        );

        assert_eq!(
            result.map.keys().copied().collect::<BTreeSet<_>>(),
            expected.map.keys().copied().collect::<BTreeSet<_>>()
        );
    }

    #[test]
    fn test_parse_pattern_two_chars_to_fsm() {
        let pattern: &str = "ab";
        let result = parse_pattern(pattern).unwrap();

        let alphabet = Alphabet {
            symbol_mapping: BTreeMap::from([('\0', 0), ('a', 1), ('b', 2)]),
            by_transition: BTreeMap::from([(0, vec!['\0']), (1, vec!['a']), (2, vec!['b'])]),
        };

        let result = result.to_fsm(Some(alphabet.clone()), None, None);

        let expected = Fsm {
            alphabet,
            states: BTreeSet::from([0, 1, 2]),
            initial: 0,
            finals: BTreeSet::from([2]),
            map: BTreeMap::from([
                (0, BTreeMap::from([(1, 1)])),
                (1, BTreeMap::from([(2, 2)])),
                (2, BTreeMap::new()),
            ]),
        };

        assert_eq!(
            result
                .alphabet
                .symbol_mapping
                .keys()
                .copied()
                .collect::<BTreeSet<_>>(),
            expected
                .alphabet
                .symbol_mapping
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
        );

        assert_eq!(
            result
                .alphabet
                .by_transition
                .keys()
                .copied()
                .collect::<BTreeSet<_>>(),
            expected
                .alphabet
                .by_transition
                .keys()
                .copied()
                .collect::<BTreeSet<_>>()
        );

        assert_eq!(
            result.states.iter().copied().collect::<BTreeSet<_>>(),
            expected.states.iter().copied().collect::<BTreeSet<_>>()
        );

        assert_eq!(result.initial, expected.initial);

        assert_eq!(
            result.finals.iter().copied().collect::<BTreeSet<_>>(),
            expected.finals.iter().copied().collect::<BTreeSet<_>>()
        );

        assert_eq!(
            result.map.keys().copied().collect::<BTreeSet<_>>(),
            expected.map.keys().copied().collect::<BTreeSet<_>>()
        );
    }

    #[test]
    fn test_parse_pattern_to_fms() {
        let test_cases = vec![
            (
                "a",
                Fsm {
                    alphabet: Alphabet {
                        symbol_mapping: BTreeMap::from([('\0', 0), ('a', 1)]),
                        by_transition: BTreeMap::from([(0, vec!['\0']), (1, vec!['a'])]),
                    },
                    states: BTreeSet::from([0, 1]),
                    initial: 0,
                    finals: BTreeSet::from([1]),
                    map: BTreeMap::from([(0, BTreeMap::from([(1, 1)])), (1, BTreeMap::new())]),
                },
            ),
            (
                "ab",
                Fsm {
                    alphabet: Alphabet {
                        symbol_mapping: BTreeMap::from([('\0', 0), ('a', 1), ('b', 2)]),
                        by_transition: BTreeMap::from([
                            (0, vec!['\0']),
                            (1, vec!['a']),
                            (2, vec!['b']),
                        ]),
                    },
                    states: BTreeSet::from([0, 1, 2]),
                    initial: 0,
                    finals: BTreeSet::from([2]),
                    map: BTreeMap::from([
                        (0, BTreeMap::from([(1, 1)])),
                        (1, BTreeMap::from([(2, 2)])),
                        (2, BTreeMap::new()),
                    ]),
                },
            ),
            (
                "a|b",
                Fsm {
                    alphabet: Alphabet {
                        symbol_mapping: BTreeMap::from([('\0', 0), ('a', 1), ('b', 2)]),
                        by_transition: BTreeMap::from([
                            (0, vec!['\0']),
                            (1, vec!['a']),
                            (2, vec!['b']),
                        ]),
                    },
                    states: BTreeSet::from([0, 1, 2]),
                    initial: 0,
                    finals: BTreeSet::from([1, 2]),
                    map: BTreeMap::from([
                        (0, BTreeMap::from([(1, 1), (2, 2)])),
                        (1, BTreeMap::new()),
                        (2, BTreeMap::new()),
                    ]),
                },
            ),
            (
                "[ab]",
                Fsm {
                    alphabet: Alphabet {
                        symbol_mapping: BTreeMap::from([('\0', 0), ('a', 1), ('b', 1)]),
                        by_transition: BTreeMap::from([(0, vec!['\0']), (1, vec!['a', 'b'])]),
                    },
                    states: BTreeSet::from([0, 1]),
                    initial: 0,
                    finals: BTreeSet::from([1]),
                    map: BTreeMap::from([(0, BTreeMap::from([(1, 1)])), (1, BTreeMap::new())]),
                },
            ),
            (
                "aaaaa",
                Fsm {
                    alphabet: Alphabet {
                        symbol_mapping: BTreeMap::from([('\0', 0), ('a', 1)]),
                        by_transition: BTreeMap::from([(0, vec!['\0']), (1, vec!['a'])]),
                    },
                    states: BTreeSet::from([0, 1, 2, 3, 4, 5]),
                    initial: 0,
                    finals: BTreeSet::from([5]),
                    map: BTreeMap::from([
                        (0, BTreeMap::from([(1, 1)])),
                        (1, BTreeMap::from([(1, 2)])),
                        (2, BTreeMap::from([(1, 3)])),
                        (3, BTreeMap::from([(1, 4)])),
                        (4, BTreeMap::from([(1, 5)])),
                        (5, BTreeMap::new()),
                    ]),
                },
            ),
            (
                "davidholtz",
                Fsm {
                    alphabet: Alphabet {
                        symbol_mapping: BTreeMap::from([
                            ('\0', 0),
                            ('a', 2),
                            ('d', 1),
                            ('h', 5),
                            ('i', 4),
                            ('l', 7),
                            ('o', 6),
                            ('t', 8),
                            ('v', 3),
                            ('z', 9),
                        ]),
                        by_transition: BTreeMap::from([
                            (0, vec!['\0']),
                            (2, vec!['a']),
                            (1, vec!['d']),
                            (5, vec!['h']),
                            (4, vec!['i']),
                            (7, vec!['l']),
                            (6, vec!['o']),
                            (8, vec!['t']),
                            (3, vec!['v']),
                            (9, vec!['z']),
                        ]),
                    },
                    states: BTreeSet::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
                    initial: 0,
                    finals: BTreeSet::from([10]),
                    map: BTreeMap::from([
                        (0, BTreeMap::from([(1, 1)])),
                        (1, BTreeMap::from([(2, 2)])),
                        (2, BTreeMap::from([(3, 3)])),
                        (3, BTreeMap::from([(4, 4)])),
                        (4, BTreeMap::from([(1, 5)])),
                        (5, BTreeMap::from([(5, 6)])),
                        (6, BTreeMap::from([(6, 7)])),
                        (7, BTreeMap::from([(7, 8)])),
                        (8, BTreeMap::from([(8, 9)])),
                        (9, BTreeMap::from([(9, 10)])),
                        (10, BTreeMap::new()),
                    ]),
                },
            ),
            (
                "a*b",
                Fsm {
                    alphabet: Alphabet {
                        symbol_mapping: BTreeMap::from([('\0', 0), ('a', 1), ('b', 2)]),
                        by_transition: BTreeMap::from([
                            (0, vec!['\0']),
                            (1, vec!['a']),
                            (2, vec!['b']),
                        ]),
                    },
                    states: BTreeSet::from([0, 1, 2]),
                    initial: 0,
                    finals: BTreeSet::from([2]),
                    map: BTreeMap::from([
                        (0, BTreeMap::from([(1, 1), (2, 2)])),
                        (1, BTreeMap::from([(1, 1), (2, 2)])),
                        (2, BTreeMap::new()),
                    ]),
                },
            ),
            (
                "(ab|cd)*",
                Fsm {
                    alphabet: Alphabet {
                        symbol_mapping: BTreeMap::from([
                            ('\0', 0),
                            ('a', 1),
                            ('b', 2),
                            ('c', 3),
                            ('d', 4),
                        ]),
                        by_transition: BTreeMap::from([
                            (0, vec!['\0']),
                            (1, vec!['a']),
                            (2, vec!['b']),
                            (3, vec!['c']),
                            (4, vec!['d']),
                        ]),
                    },
                    states: BTreeSet::from([0, 1, 2, 3, 4]),
                    initial: 0,
                    finals: BTreeSet::from([0, 3, 4]),
                    map: BTreeMap::from([
                        (0, BTreeMap::from([(1, 1), (3, 2)])),
                        (1, BTreeMap::from([(2, 3)])),
                        (2, BTreeMap::from([(4, 4)])),
                        (3, BTreeMap::from([(1, 1), (3, 2)])),
                        (4, BTreeMap::from([(1, 1), (3, 2)])),
                    ]),
                },
            ),
            (
                "[a-d]",
                Fsm {
                    alphabet: Alphabet {
                        symbol_mapping: BTreeMap::from([
                            ('\0', 0),
                            ('a', 1),
                            ('b', 1),
                            ('c', 1),
                            ('d', 1),
                        ]),
                        by_transition: BTreeMap::from([
                            (0, vec!['\0']),
                            (1, vec!['a', 'b', 'c', 'd']),
                        ]),
                    },
                    states: BTreeSet::from([0, 1]),
                    initial: 0,
                    finals: BTreeSet::from([1]),
                    map: BTreeMap::from([(0, BTreeMap::from([(1, 1)])), (1, BTreeMap::new())]),
                },
            ),
            (
                "[a-z0-9]+",
                Fsm {
                    alphabet: Alphabet {
                        symbol_mapping: BTreeMap::from([
                            ('\0', 0),
                            ('0', 1),
                            ('1', 1),
                            ('2', 1),
                            ('3', 1),
                            ('4', 1),
                            ('5', 1),
                            ('6', 1),
                            ('7', 1),
                            ('8', 1),
                            ('9', 1),
                            ('a', 1),
                            ('b', 1),
                            ('c', 1),
                            ('d', 1),
                            ('e', 1),
                            ('f', 1),
                            ('g', 1),
                            ('h', 1),
                            ('i', 1),
                            ('j', 1),
                            ('k', 1),
                            ('l', 1),
                            ('m', 1),
                            ('n', 1),
                            ('o', 1),
                            ('p', 1),
                            ('q', 1),
                            ('r', 1),
                            ('s', 1),
                            ('t', 1),
                            ('u', 1),
                            ('v', 1),
                            ('w', 1),
                            ('x', 1),
                            ('y', 1),
                            ('z', 1),
                        ]),
                        by_transition: BTreeMap::from([
                            (0, vec!['\0']),
                            (
                                1,
                                vec![
                                    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b',
                                    'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n',
                                    'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
                                ],
                            ),
                        ]),
                    },
                    states: BTreeSet::from([0, 1, 2]),
                    initial: 0,
                    finals: BTreeSet::from([1, 2]),
                    map: BTreeMap::from([
                        (0, BTreeMap::from([(1, 1)])),
                        (1, BTreeMap::from([(1, 2)])),
                        (2, BTreeMap::from([(1, 2)])),
                    ]),
                },
            ),
            // (
            //     "c?",
            //     Fsm {
            //         alphabet: Alphabet {
            //             symbol_mapping: BTreeMap::from([('\0', 0), ('c', 1)]),
            //             by_transition: BTreeMap::from([(0, vec!['\0']), (1, vec!['c'])]),
            //         },
            //         states: BTreeSet::from([0, 1]),
            //         initial: 0,
            //         finals: BTreeSet::from([1]),
            //         map: BTreeMap::from([(0, BTreeMap::from([(1, 1)])), (1, BTreeMap::new())]),
            //     },
            // ),
        ];

        for (pattern, expected) in test_cases {
            let fsm = parse_pattern_to_fms(pattern);

            println!("\n\n\nPattern: {}", pattern);
            println!("Generated FSM: {:?}", fsm);
            println!("Expected  FSM: {:?}", expected);

            for (state, transitions) in fsm.map.iter() {
                for (symbol, next_state) in transitions.iter() {
                    assert!(
                        expected.map[state].contains_key(symbol),
                        "State {} does not contain symbol {}",
                        state,
                        symbol
                    );
                    assert_eq!(
                        expected.map[state][symbol], *next_state,
                        "State {} does not transition to the expected state for symbol {}",
                        state, symbol
                    );
                }
            }
            assert_eq!(fsm.states, expected.states);
            assert_eq!(fsm.initial, expected.initial);
            assert_eq!(fsm.finals, expected.finals);
            assert_eq!(fsm.map, expected.map);
        }
    }
}
