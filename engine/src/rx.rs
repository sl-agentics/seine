//! Tiny backtracking regex engine for the `matches` operator subset.
//!
//! Supported (D-030): literal chars, `.`, character classes with ranges and
//! `^` negation, groups, alternation `|`, and the `* + ?` quantifiers.
//! Everything else — `{n,m}`, backslash escapes, anchors, backreferences,
//! lookaround — is a parse error, keeping the subset wall mechanical.
//!
//! Acceptance is FULL-STRING (java.util.regex `String.matches` semantics,
//! op_m1/m2/m5). For this feature set (no backrefs/lookaround) backtracking
//! and NFA semantics agree on acceptance, and greediness is unobservable,
//! so this matcher is equivalent to Java's for the in-subset alphabet
//! (ASCII, no newlines — pr09/D-010 corpus constraint).

#[derive(Debug, Clone, PartialEq)]
enum Rx {
    Char(char),
    Any,
    Class { neg: bool, items: Vec<(char, char)> },
    Seq(Vec<Rx>),
    Alt(Vec<Rx>),
    Star(Box<Rx>),
    Plus(Box<Rx>),
    Opt(Box<Rx>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Regex {
    ast: Rx,
    src: String,
}

impl Regex {
    pub fn parse(src: &str) -> Result<Regex, String> {
        let chars: Vec<char> = src.chars().collect();
        let mut p = RxParser { chars, pos: 0 };
        let ast = p.alt()?;
        if p.pos != p.chars.len() {
            return Err(format!(
                "regex {src:?}: unexpected {:?} at {}",
                p.chars[p.pos], p.pos
            ));
        }
        Ok(Regex { ast, src: src.to_string() })
    }

    pub fn source(&self) -> &str {
        &self.src
    }

    /// Full-string acceptance.
    pub fn accepts(&self, s: &str) -> bool {
        let cs: Vec<char> = s.chars().collect();
        m(&self.ast, &cs, 0, &|i| i == cs.len())
    }
}

struct RxParser {
    chars: Vec<char>,
    pos: usize,
}

impl RxParser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn alt(&mut self) -> Result<Rx, String> {
        let mut branches = vec![self.seq()?];
        while self.peek() == Some('|') {
            self.pos += 1;
            branches.push(self.seq()?);
        }
        Ok(if branches.len() == 1 {
            branches.pop().unwrap()
        } else {
            Rx::Alt(branches)
        })
    }

    fn seq(&mut self) -> Result<Rx, String> {
        let mut items = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            items.push(self.piece()?);
        }
        Ok(if items.len() == 1 { items.pop().unwrap() } else { Rx::Seq(items) })
    }

    fn piece(&mut self) -> Result<Rx, String> {
        let atom = self.atom()?;
        Ok(match self.peek() {
            Some('*') => {
                self.pos += 1;
                Rx::Star(Box::new(atom))
            }
            Some('+') => {
                self.pos += 1;
                Rx::Plus(Box::new(atom))
            }
            Some('?') => {
                self.pos += 1;
                Rx::Opt(Box::new(atom))
            }
            _ => atom,
        })
    }

    fn atom(&mut self) -> Result<Rx, String> {
        let c = self.peek().ok_or("regex: unexpected end")?;
        match c {
            '.' => {
                self.pos += 1;
                Ok(Rx::Any)
            }
            '(' => {
                self.pos += 1;
                let inner = self.alt()?;
                if self.peek() != Some(')') {
                    return Err("regex: unclosed group".into());
                }
                self.pos += 1;
                Ok(inner)
            }
            '[' => {
                self.pos += 1;
                self.class()
            }
            '*' | '+' | '?' => Err(format!("regex: dangling quantifier {c:?}")),
            ')' | ']' => Err(format!("regex: unmatched {c:?}")),
            '\\' | '{' | '}' | '^' | '$' => {
                Err(format!("regex: unsupported metacharacter {c:?} (subset wall)"))
            }
            other => {
                self.pos += 1;
                Ok(Rx::Char(other))
            }
        }
    }

    fn class(&mut self) -> Result<Rx, String> {
        let neg = if self.peek() == Some('^') {
            self.pos += 1;
            true
        } else {
            false
        };
        let mut items = Vec::new();
        loop {
            let c = self.peek().ok_or("regex: unclosed class")?;
            if c == ']' {
                if items.is_empty() {
                    return Err("regex: empty class".into());
                }
                self.pos += 1;
                return Ok(Rx::Class { neg, items });
            }
            if c == '\\' || c == '[' {
                return Err(format!("regex: unsupported {c:?} in class (subset wall)"));
            }
            self.pos += 1;
            if self.peek() == Some('-') && self.chars.get(self.pos + 1) != Some(&']') {
                self.pos += 1;
                let hi = self.peek().ok_or("regex: unclosed class range")?;
                self.pos += 1;
                if hi < c {
                    return Err(format!("regex: inverted range {c}-{hi}"));
                }
                items.push((c, hi));
            } else {
                items.push((c, c));
            }
        }
    }
}

/// Backtracking matcher in continuation-passing style; `k` receives the
/// position after this node consumed input. The `j > i` progress guard in
/// Star/Plus keeps empty-matching inners (e.g. `(a?)*`) terminating.
fn m(rx: &Rx, s: &[char], i: usize, k: &dyn Fn(usize) -> bool) -> bool {
    match rx {
        Rx::Char(c) => i < s.len() && s[i] == *c && k(i + 1),
        Rx::Any => i < s.len() && k(i + 1),
        Rx::Class { neg, items } => {
            i < s.len() && {
                let inside = items.iter().any(|(lo, hi)| *lo <= s[i] && s[i] <= *hi);
                inside != *neg && k(i + 1)
            }
        }
        Rx::Seq(items) => m_seq(items, s, i, k),
        Rx::Alt(branches) => branches.iter().any(|b| m(b, s, i, k)),
        Rx::Star(inner) => m_star(inner, s, i, k),
        Rx::Plus(inner) => m(inner, s, i, &|j| m_star(inner, s, j, k)),
        Rx::Opt(inner) => m(inner, s, i, k) || k(i),
    }
}

fn m_seq(items: &[Rx], s: &[char], i: usize, k: &dyn Fn(usize) -> bool) -> bool {
    match items.split_first() {
        None => k(i),
        Some((first, rest)) => m(first, s, i, &|j| m_seq(rest, s, j, k)),
    }
}

fn m_star(inner: &Rx, s: &[char], i: usize, k: &dyn Fn(usize) -> bool) -> bool {
    m(inner, s, i, &|j| j > i && m_star(inner, s, j, k)) || k(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(pat: &str, s: &str) -> bool {
        Regex::parse(pat).unwrap().accepts(s)
    }

    #[test]
    fn matches_oracle_pinned_cases() {
        // op_m1/m2: full-string anchoring
        assert!(ok("a.*", "abc") && ok("a.*", "a") && ok("a.*", "ab"));
        assert!(!ok("a.*", "xbc") && !ok("a.*", ""));
        assert!(ok("b", "b") && !ok("b", "abc"));
        assert!(ok(".*b.*", "abc") && ok(".*b.*", "b"));
        // op_m4: classes, alternation, groups, quantifiers
        for s in ["aac", "ab", "abc", "a"] {
            assert!(ok("[ab]+c?", s), "{s}");
        }
        assert!(!ok("[ab]+c?", "bqc") && !ok("[ab]+c?", "zz"));
        assert!(ok("a|zz", "a") && ok("a|zz", "zz") && !ok("a|zz", "az"));
        assert!(ok("x(y|z)*", "xyzzy") && ok("x(y|z)*", "x") && !ok("x(y|z)*", "xq"));
        assert!(ok("[a-c]q[^a]", "bqc") && !ok("[a-c]q[^a]", "bqa") && !ok("[a-c]q[^a]", "xqc"));
        // op_m5: dot and empty pattern
        assert!(ok(".*", "") && ok(".*", "q") && ok(".*", "qq"));
        assert!(ok(".", "q") && !ok(".", "") && !ok(".", "qq"));
        assert!(ok("", "") && !ok("", "q"));
    }

    #[test]
    fn terminates_on_empty_matching_star() {
        assert!(ok("(a?)*", "aaa"));
        assert!(ok("(a?)*", ""));
        assert!(!ok("(a?)*b", "c"));
    }

    #[test]
    fn rejects_out_of_subset() {
        for pat in ["a{2}", "\\d", "^a", "a$", "[\\d]", "(a", "a)", "*a", "[]", "[z-a]"] {
            assert!(Regex::parse(pat).is_err(), "{pat} should be rejected");
        }
    }
}
