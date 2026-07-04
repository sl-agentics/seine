//! Parser for the supported DRL subset.
//!
//! Grammar (grows strictly by phase; anything outside is a parse error so the
//! scope wall is enforced mechanically):
//!
//! ```text
//! file       := rule*
//! rule       := "rule" name attr* "when" pattern* "then" action* "end"
//! name       := STRING | IDENT
//! attr       := "salience" ["-"] INT | "no-loop" [BOOL]
//!             | "salience" "(" salterm [("+"|"-"|"*") salterm] ")"
//! salterm    := ["-"] INT | "$id"      (numeric bindings only — D-043;
//!               method calls / full MVEL salience bodies out of subset)
//! pattern    := ["not"|"exists"] [ "$id" ":" ] IDENT "(" [constraint ("," constraint)*] ")"
//!               (bindings are rejected inside not/exists patterns — D-031)
//!             | "accumulate" "(" pattern ";" "$id" ":" accfunc "(" ["$id"] ")" ")"
//!             | "$id" ":" ("List"|"ArrayList"|"Collection") "(" ")" "from" "collect" "(" pattern ")"
//! accfunc    := "sum" | "count" | "average" | "min" | "max"
//!               (custom/multi-function accumulates and `from accumulate`
//!                are out of subset — D-038)
//! constraint := "$id" ":" IDENT            (field binding)
//!             | IDENT cmpop (literal|"$id") (field test, RHS literal or binding)
//!             | IDENT "matches" STRING      (literal regex, String fields)
//!             | IDENT "contains" STRING     (literal substring, String fields)
//!             | IDENT ["not"] "in" "(" literal ("," literal)* ")"
//! cmpop      := "==" | "!=" | "<" | "<=" | ">" | ">="
//! literal    := ["-"] INT | ["-"] FLOAT | STRING | "true" | "false"
//! action     := "insert" "(" "new" IDENT "(" [arg ("," arg)*] ")" ")" ";"
//!             | "$id" "." "set" IDENT "(" arg ")" ";"
//!             | "update" "(" "$id" ")" ";"
//!             | ("delete"|"retract") "(" "$id" ")" ";"
//!             | "modify" "(" "$id" ")" "{" [ "set"IDENT "(" arg ")" ("," ...)* ] "}"
//!               (desugars to setters followed by update)
//! arg        := literal | "$id" | "$id" "." ("get"|"is") IDENT "(" ")"
//! ```

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    StrLit(String),
    IntLit(i64),
    FloatLit(f64),
    Sym(&'static str),
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Ident(s) => write!(f, "{s}"),
            Tok::StrLit(s) => write!(f, "{s:?}"),
            Tok::IntLit(n) => write!(f, "{n}"),
            Tok::FloatLit(n) => write!(f, "{n}"),
            Tok::Sym(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    I64(i64),
    F64(f64),
    Str(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CmpRhs {
    Lit(Literal),
    /// A field binding declared earlier (same or previous pattern).
    Var(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// `$a : age`
    Bind { var: String, field: String },
    /// `age > 18` or `age > $a`
    Cmp { field: String, op: CmpOp, rhs: CmpRhs },
    /// `name matches "a.*"` — full-string java.util.regex semantics (D-030)
    Matches { field: String, regex: String },
    /// `name contains "ab"` — String substring test (D-030)
    Contains { field: String, needle: String },
    /// `n in (1, 2)` / `n not in (1, 2)` — literal membership (D-030)
    InList { field: String, items: Vec<Literal>, negated: bool },
}

/// Conditional-element kind of a pattern (D-031).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeKind {
    Positive,
    Not,
    Exists,
}

/// Built-in accumulate functions (D-038). Collect is `from collect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccFunc {
    Sum,
    Count,
    Average,
    Min,
    Max,
    Collect,
}

/// Inline accumulate / collect spec attached to a pattern whose
/// type/constraints describe the SOURCE (D-038). The source's field
/// bindings are scoped inside; only `arg` may be bound there.
#[derive(Debug, Clone, PartialEq)]
pub struct AccSpec {
    pub func: AccFunc,
    /// The source binding accumulated over (None for count()/collect).
    pub arg: Option<String>,
    /// The result binding, visible downstream.
    pub result_var: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub binding: Option<String>,
    pub type_name: String,
    pub constraints: Vec<Constraint>,
    pub ce: CeKind,
    /// Some(_) makes this an accumulate/collect CE over the source
    /// described by type_name/constraints (D-038).
    pub acc: Option<AccSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RhsArg {
    Lit(Literal),
    /// `$a` — a field binding from the LHS
    Var(String),
    /// `$p.getName()` — getter on a fact binding; resolved to field `name`
    Getter { var: String, field: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Insert { type_name: String, args: Vec<RhsArg> },
    /// `$p.setX(arg);` — mutates immediately, contributes X to the pending
    /// modification mask consumed by the next `update($p)`.
    Set { var: String, field: String, arg: RhsArg },
    /// `update($p);`
    Update { var: String },
    /// `delete($p);` / `retract($p);`
    Delete { var: String },
}

/// One term of a salience expression (D-043).
#[derive(Debug, Clone, PartialEq)]
pub enum SalTerm {
    Lit(i64),
    Var(String),
}

/// Rule salience: a static int, or a computed expression over LHS
/// bindings — evaluated per activation (D-043).
#[derive(Debug, Clone, PartialEq)]
pub enum SalienceSpec {
    Static(i64),
    Expr { a: SalTerm, op: Option<(char, SalTerm)> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleDef {
    pub name: String,
    pub salience: SalienceSpec,
    pub no_loop: bool,
    pub patterns: Vec<Pattern>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrlError(pub String);

impl fmt::Display for DrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DRL parse error: {}", self.0)
    }
}

fn lex(src: &str) -> Result<Vec<Tok>, DrlError> {
    let b: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            if i + 1 >= b.len() {
                return Err(DrlError("unterminated block comment".into()));
            }
            i += 2;
        } else if c.is_ascii_alphabetic() || c == '_' || c == '$' {
            let start = i;
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == '_') {
                i += 1;
            }
            let mut word: String = b[start..i].iter().collect();
            // `no-loop` is one keyword: an ident "no" immediately followed by "-loop"
            if word == "no" && i + 4 < b.len() && b[i..i + 5].iter().collect::<String>() == "-loop"
            {
                word = "no-loop".into();
                i += 5;
            }
            out.push(Tok::Ident(word));
        } else if c.is_ascii_digit() {
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < b.len() && b[i] == '.' && b[i + 1].is_ascii_digit() {
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                let s: String = b[start..i].iter().collect();
                out.push(Tok::FloatLit(s.parse().map_err(|e| {
                    DrlError(format!("bad float literal {s}: {e}"))
                })?));
            } else {
                let s: String = b[start..i].iter().collect();
                out.push(Tok::IntLit(s.parse().map_err(|e| {
                    DrlError(format!("bad int literal {s}: {e}"))
                })?));
            }
        } else if c == '"' {
            i += 1;
            let mut s = String::new();
            loop {
                if i >= b.len() {
                    return Err(DrlError("unterminated string literal".into()));
                }
                match b[i] {
                    '"' => {
                        i += 1;
                        break;
                    }
                    '\\' => {
                        i += 1;
                        if i >= b.len() {
                            return Err(DrlError("unterminated escape".into()));
                        }
                        s.push(match b[i] {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '"' => '"',
                            other => {
                                return Err(DrlError(format!("unsupported escape \\{other}")))
                            }
                        });
                        i += 1;
                    }
                    other => {
                        s.push(other);
                        i += 1;
                    }
                }
            }
            out.push(Tok::StrLit(s));
        } else {
            let two: String = b[i..(i + 2).min(b.len())].iter().collect();
            let sym: &'static str = match two.as_str() {
                "==" => "==",
                "!=" => "!=",
                "<=" => "<=",
                ">=" => ">=",
                _ => match c {
                    '(' => "(",
                    ')' => ")",
                    '{' => "{",
                    '}' => "}",
                    ',' => ",",
                    ';' => ";",
                    ':' => ":",
                    '.' => ".",
                    '<' => "<",
                    '>' => ">",
                    '-' => "-",
                    '+' => "+",
                    '*' => "*",
                    other => return Err(DrlError(format!("unexpected character {other:?}"))),
                },
            };
            i += sym.len();
            out.push(Tok::Sym(sym));
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Result<Tok, DrlError> {
        let t = self
            .toks
            .get(self.pos)
            .cloned()
            .ok_or_else(|| DrlError("unexpected end of input".into()))?;
        self.pos += 1;
        Ok(t)
    }

    fn expect_sym(&mut self, s: &str) -> Result<(), DrlError> {
        match self.next()? {
            Tok::Sym(x) if x == s => Ok(()),
            other => Err(DrlError(format!("expected {s:?}, got {other}"))),
        }
    }

    fn expect_kw(&mut self, kw: &str) -> Result<(), DrlError> {
        match self.next()? {
            Tok::Ident(x) if x == kw => Ok(()),
            other => Err(DrlError(format!("expected keyword {kw:?}, got {other}"))),
        }
    }

    fn at_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(x)) if x == kw)
    }

    fn ident(&mut self) -> Result<String, DrlError> {
        match self.next()? {
            Tok::Ident(x) => Ok(x),
            other => Err(DrlError(format!("expected identifier, got {other}"))),
        }
    }

    fn literal(&mut self) -> Result<Literal, DrlError> {
        match self.next()? {
            Tok::IntLit(n) => Ok(Literal::I64(n)),
            Tok::FloatLit(n) => Ok(Literal::F64(n)),
            Tok::StrLit(s) => Ok(Literal::Str(s)),
            Tok::Ident(w) if w == "true" => Ok(Literal::Bool(true)),
            Tok::Ident(w) if w == "false" => Ok(Literal::Bool(false)),
            Tok::Sym("-") => match self.next()? {
                Tok::IntLit(n) => Ok(Literal::I64(-n)),
                Tok::FloatLit(n) => Ok(Literal::F64(-n)),
                other => Err(DrlError(format!("expected number after '-', got {other}"))),
            },
            other => Err(DrlError(format!("expected literal, got {other}"))),
        }
    }

    fn rule(&mut self) -> Result<RuleDef, DrlError> {
        self.expect_kw("rule")?;
        let name = match self.next()? {
            Tok::StrLit(s) => s,
            Tok::Ident(s) => s,
            other => Err(DrlError(format!("expected rule name, got {other}")))?,
        };
        let mut salience = SalienceSpec::Static(0);
        let mut no_loop = false;
        loop {
            if self.at_kw("salience") {
                self.next()?;
                if matches!(self.peek(), Some(Tok::Sym("("))) {
                    self.next()?;
                    let a = self.sal_term()?;
                    let op = match self.peek() {
                        Some(Tok::Sym(o @ ("+" | "-" | "*"))) => {
                            let c = o.chars().next().unwrap();
                            self.next()?;
                            Some((c, self.sal_term()?))
                        }
                        _ => None,
                    };
                    self.expect_sym(")")?;
                    salience = SalienceSpec::Expr { a, op };
                } else {
                    salience = match self.literal()? {
                        Literal::I64(n) => SalienceSpec::Static(n),
                        other => {
                            return Err(DrlError(format!(
                                "salience must be an int or (expr), got {other:?}"
                            )))
                        }
                    };
                }
            } else if self.at_kw("no-loop") {
                self.next()?;
                no_loop = true;
                if self.at_kw("true") || self.at_kw("false") {
                    no_loop = self.at_kw("true");
                    self.next()?;
                }
            } else if self.at_kw("when") {
                self.next()?;
                break;
            } else {
                return Err(DrlError(format!(
                    "expected rule attribute or 'when', got {:?}",
                    self.peek().map(|t| t.to_string())
                )));
            }
        }
        let mut patterns = Vec::new();
        while !self.at_kw("then") {
            patterns.push(self.pattern()?);
        }
        self.expect_kw("then")?;
        let mut actions = Vec::new();
        while !self.at_kw("end") {
            actions.extend(self.actions()?);
        }
        self.expect_kw("end")?;
        Ok(RuleDef { name, salience, no_loop, patterns, actions })
    }

    /// One salience-expression term: int literal or `$binding` (D-043).
    /// Anything else (method calls, floats, parens) is out of subset.
    fn sal_term(&mut self) -> Result<SalTerm, DrlError> {
        match self.peek() {
            Some(Tok::IntLit(_)) | Some(Tok::Sym("-")) => match self.literal()? {
                Literal::I64(n) => Ok(SalTerm::Lit(n)),
                other => Err(DrlError(format!(
                    "salience terms are int literals or bindings, got {other:?}"
                ))),
            },
            Some(Tok::Ident(w)) if w.starts_with('$') => Ok(SalTerm::Var(self.dollar_ident()?)),
            other => Err(DrlError(format!(
                "salience terms are int literals or bindings, got {:?}",
                other.map(|t| t.to_string())
            ))),
        }
    }

    fn pattern(&mut self) -> Result<Pattern, DrlError> {
        let ce = if self.at_kw("not") {
            self.next()?;
            CeKind::Not
        } else if self.at_kw("exists") {
            self.next()?;
            CeKind::Exists
        } else {
            CeKind::Positive
        };
        if self.at_kw("accumulate") {
            if ce != CeKind::Positive {
                return Err(DrlError("not/exists over accumulate not in subset".into()));
            }
            return self.accumulate_pattern();
        }
        let first = self.ident()?;
        let (binding, type_name) = if first.starts_with('$') {
            self.expect_sym(":")?;
            (Some(first), self.ident()?)
        } else {
            (None, first)
        };
        self.expect_sym("(")?;
        let mut constraints = Vec::new();
        if !matches!(self.peek(), Some(Tok::Sym(")"))) {
            loop {
                constraints.push(self.constraint()?);
                match self.next()? {
                    Tok::Sym(",") => continue,
                    Tok::Sym(")") => break,
                    other => {
                        return Err(DrlError(format!("expected ',' or ')', got {other}")))
                    }
                }
            }
        } else {
            self.next()?;
        }
        if self.at_kw("from") {
            self.next()?;
            if !self.at_kw("collect") {
                return Err(DrlError(
                    "`from` is only supported as `from collect` (D-038)".into(),
                ));
            }
            self.next()?;
            if ce != CeKind::Positive {
                return Err(DrlError("not/exists over collect not in subset".into()));
            }
            if !matches!(type_name.as_str(), "List" | "ArrayList" | "Collection") {
                return Err(DrlError(format!(
                    "collect result pattern must be List/ArrayList/Collection, got {type_name}"
                )));
            }
            if !constraints.is_empty() {
                return Err(DrlError(
                    "constraints on the collect result pattern are not in subset".into(),
                ));
            }
            let result_var = binding
                .ok_or_else(|| DrlError("collect result must be bound (`$l : List()`)".into()))?;
            self.expect_sym("(")?;
            let src = self.pattern()?;
            self.expect_sym(")")?;
            if src.ce != CeKind::Positive || src.acc.is_some() {
                return Err(DrlError("collect source must be a plain pattern".into()));
            }
            if src.binding.is_some()
                || src.constraints.iter().any(|c| matches!(c, Constraint::Bind { .. }))
            {
                return Err(DrlError(
                    "bindings inside a collect source are not in subset".into(),
                ));
            }
            // A collect source referencing outer bindings builds an RIA
            // SUBNETWORK — unported territory with its own quirks
            // (D-041/fz_999_4371): alpha-only sources stay in subset.
            if src.constraints.iter().any(
                |c| matches!(c, Constraint::Cmp { rhs: CmpRhs::Var(_), .. }),
            ) {
                return Err(DrlError(
                    "variable references inside a collect source are not in subset (subnetwork, D-041)".into(),
                ));
            }
            return Ok(Pattern {
                binding: None,
                type_name: src.type_name,
                constraints: src.constraints,
                ce: CeKind::Positive,
                acc: Some(AccSpec { func: AccFunc::Collect, arg: None, result_var }),
            });
        }
        if ce != CeKind::Positive {
            // Bindings inside not/exists are scoped out in Drools; the
            // subset rejects them outright (D-031).
            if binding.is_some()
                || constraints.iter().any(|c| matches!(c, Constraint::Bind { .. }))
            {
                return Err(DrlError(
                    "bindings are not allowed in not/exists patterns".into(),
                ));
            }
        }
        Ok(Pattern { binding, type_name, constraints, ce, acc: None })
    }

    /// `accumulate( <source pattern> ; $r : func([$arg]) )` — built-in
    /// functions only; multi-function and custom (init/action/result)
    /// accumulates are out of subset (D-038).
    fn accumulate_pattern(&mut self) -> Result<Pattern, DrlError> {
        self.expect_kw("accumulate")?;
        self.expect_sym("(")?;
        let src = self.pattern()?;
        if src.ce != CeKind::Positive || src.acc.is_some() || src.binding.is_some() {
            return Err(DrlError(
                "accumulate source must be a plain unbound pattern".into(),
            ));
        }
        self.expect_sym(";")?;
        let result_var = self.dollar_ident()?;
        self.expect_sym(":")?;
        let fname = self.ident()?;
        let func = match fname.as_str() {
            "sum" => AccFunc::Sum,
            "count" => AccFunc::Count,
            "average" => AccFunc::Average,
            "min" => AccFunc::Min,
            "max" => AccFunc::Max,
            other => {
                return Err(DrlError(format!(
                    "accumulate function {other:?} not in subset (built-ins only: sum/count/average/min/max)"
                )))
            }
        };
        self.expect_sym("(")?;
        let arg = if matches!(self.peek(), Some(Tok::Sym(")"))) {
            None
        } else {
            Some(self.dollar_ident()?)
        };
        self.expect_sym(")")?;
        if matches!(self.peek(), Some(Tok::Sym(","))) {
            return Err(DrlError("multi-function accumulate not in subset".into()));
        }
        self.expect_sym(")")?;
        if func != AccFunc::Count && arg.is_none() {
            return Err(DrlError(format!("{fname} requires a bound argument")));
        }
        // source bindings are scoped inside the accumulate; the arg must
        // be one of them (unused extras are legal and simply ignored)
        if let Some(a) = &arg {
            if !src
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::Bind { var, .. } if var == a))
            {
                return Err(DrlError(format!("unknown accumulate argument {a}")));
            }
        }
        Ok(Pattern {
            binding: None,
            type_name: src.type_name,
            constraints: src.constraints,
            ce: CeKind::Positive,
            acc: Some(AccSpec { func, arg, result_var }),
        })
    }

    fn constraint(&mut self) -> Result<Constraint, DrlError> {
        let first = self.ident()?;
        if first.starts_with('$') {
            self.expect_sym(":")?;
            let field = self.ident()?;
            return Ok(Constraint::Bind { var: first, field });
        }
        match self.peek() {
            Some(Tok::Ident(w)) if w == "matches" => {
                self.next()?;
                return match self.next()? {
                    Tok::StrLit(s) => Ok(Constraint::Matches { field: first, regex: s }),
                    other => Err(DrlError(format!(
                        "matches requires a literal string regex, got {other}"
                    ))),
                };
            }
            Some(Tok::Ident(w)) if w == "contains" => {
                self.next()?;
                return match self.next()? {
                    Tok::StrLit(s) => Ok(Constraint::Contains { field: first, needle: s }),
                    other => Err(DrlError(format!(
                        "contains requires a literal string, got {other}"
                    ))),
                };
            }
            Some(Tok::Ident(w)) if w == "in" => {
                self.next()?;
                let items = self.in_list()?;
                return Ok(Constraint::InList { field: first, items, negated: false });
            }
            Some(Tok::Ident(w)) if w == "not" => {
                self.next()?;
                self.expect_kw("in")?;
                let items = self.in_list()?;
                return Ok(Constraint::InList { field: first, items, negated: true });
            }
            _ => {}
        }
        let op = match self.next()? {
            Tok::Sym("==") => CmpOp::Eq,
            Tok::Sym("!=") => CmpOp::Ne,
            Tok::Sym("<") => CmpOp::Lt,
            Tok::Sym("<=") => CmpOp::Le,
            Tok::Sym(">") => CmpOp::Gt,
            Tok::Sym(">=") => CmpOp::Ge,
            other => return Err(DrlError(format!("expected comparison operator, got {other}"))),
        };
        let rhs = match self.peek() {
            Some(Tok::Ident(w)) if w.starts_with('$') => CmpRhs::Var(self.ident()?),
            _ => CmpRhs::Lit(self.literal()?),
        };
        Ok(Constraint::Cmp { field: first, op, rhs })
    }

    fn in_list(&mut self) -> Result<Vec<Literal>, DrlError> {
        self.expect_sym("(")?;
        let mut items = vec![self.literal()?];
        loop {
            match self.next()? {
                Tok::Sym(",") => items.push(self.literal()?),
                Tok::Sym(")") => break,
                other => return Err(DrlError(format!("expected ',' or ')', got {other}"))),
            }
        }
        Ok(items)
    }

    /// Parse one RHS statement; `modify` desugars to several actions.
    fn actions(&mut self) -> Result<Vec<Action>, DrlError> {
        match self.peek() {
            Some(Tok::Ident(w)) if w == "insert" => {
                self.next()?;
                self.expect_sym("(")?;
                self.expect_kw("new")?;
                let type_name = self.ident()?;
                self.expect_sym("(")?;
                let mut args = Vec::new();
                if !matches!(self.peek(), Some(Tok::Sym(")"))) {
                    loop {
                        args.push(self.rhs_arg()?);
                        match self.next()? {
                            Tok::Sym(",") => continue,
                            Tok::Sym(")") => break,
                            other => {
                                return Err(DrlError(format!("expected ',' or ')', got {other}")))
                            }
                        }
                    }
                } else {
                    self.next()?;
                }
                self.expect_sym(")")?;
                self.expect_sym(";")?;
                Ok(vec![Action::Insert { type_name, args }])
            }
            Some(Tok::Ident(w)) if w == "update" => {
                self.next()?;
                self.expect_sym("(")?;
                let var = self.dollar_ident()?;
                self.expect_sym(")")?;
                self.expect_sym(";")?;
                Ok(vec![Action::Update { var }])
            }
            Some(Tok::Ident(w)) if w == "delete" || w == "retract" => {
                self.next()?;
                self.expect_sym("(")?;
                let var = self.dollar_ident()?;
                self.expect_sym(")")?;
                self.expect_sym(";")?;
                Ok(vec![Action::Delete { var }])
            }
            Some(Tok::Ident(w)) if w == "modify" => {
                self.next()?;
                self.expect_sym("(")?;
                let var = self.dollar_ident()?;
                self.expect_sym(")")?;
                self.expect_sym("{")?;
                let mut out = Vec::new();
                if !matches!(self.peek(), Some(Tok::Sym("}"))) {
                    loop {
                        let setter = self.ident()?;
                        let field = setter_field(&setter)?;
                        self.expect_sym("(")?;
                        let arg = self.rhs_arg()?;
                        self.expect_sym(")")?;
                        out.push(Action::Set { var: var.clone(), field, arg });
                        match self.next()? {
                            Tok::Sym(",") => continue,
                            Tok::Sym("}") => break,
                            other => {
                                return Err(DrlError(format!("expected ',' or '}}', got {other}")))
                            }
                        }
                    }
                } else {
                    self.next()?;
                }
                out.push(Action::Update { var });
                Ok(out)
            }
            Some(Tok::Ident(w)) if w.starts_with('$') => {
                let var = self.ident()?;
                self.expect_sym(".")?;
                let setter = self.ident()?;
                let field = setter_field(&setter)?;
                self.expect_sym("(")?;
                let arg = self.rhs_arg()?;
                self.expect_sym(")")?;
                self.expect_sym(";")?;
                Ok(vec![Action::Set { var, field, arg }])
            }
            other => Err(DrlError(format!(
                "expected RHS statement, got {:?}",
                other.map(|t| t.to_string())
            ))),
        }
    }

    fn dollar_ident(&mut self) -> Result<String, DrlError> {
        let id = self.ident()?;
        if id.starts_with('$') {
            Ok(id)
        } else {
            Err(DrlError(format!("expected $binding, got {id}")))
        }
    }

    fn rhs_arg(&mut self) -> Result<RhsArg, DrlError> {
        if let Some(Tok::Ident(w)) = self.peek() {
            if w.starts_with('$') {
                let var = self.ident()?;
                if matches!(self.peek(), Some(Tok::Sym("."))) {
                    self.next()?;
                    let getter = self.ident()?;
                    self.expect_sym("(")?;
                    self.expect_sym(")")?;
                    // Drools declared types generate getX() for non-boolean
                    // fields and isX() (only) for boolean fields (D-009).
                    let field = getter
                        .strip_prefix("get")
                        .or_else(|| getter.strip_prefix("is"))
                        .filter(|r| !r.is_empty())
                        .map(|r| {
                            let mut cs = r.chars();
                            let head = cs.next().unwrap().to_ascii_lowercase();
                            format!("{head}{}", cs.as_str())
                        })
                        .ok_or_else(|| {
                            DrlError(format!("unsupported method call .{getter}() (only getters)"))
                        })?;
                    return Ok(RhsArg::Getter { var, field });
                }
                return Ok(RhsArg::Var(var));
            }
        }
        Ok(RhsArg::Lit(self.literal()?))
    }
}

fn setter_field(setter: &str) -> Result<String, DrlError> {
    setter
        .strip_prefix("set")
        .filter(|r| !r.is_empty())
        .map(|r| {
            let mut cs = r.chars();
            let head = cs.next().unwrap().to_ascii_lowercase();
            format!("{head}{}", cs.as_str())
        })
        .ok_or_else(|| DrlError(format!("expected setter, got {setter}")))
}

pub fn parse_rules(src: &str) -> Result<Vec<RuleDef>, DrlError> {
    let mut p = Parser { toks: lex(src)?, pos: 0 };
    let mut rules = Vec::new();
    while p.peek().is_some() {
        rules.push(p.rule()?);
    }
    Ok(rules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_trivial_rule() {
        let rules = parse_rules(
            "rule \"Adult\"\nwhen\n    $p : Person(age > 18)\nthen\n    insert(new Adult($p.getName()));\nend\n",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        let r = &rules[0];
        assert_eq!(r.name, "Adult");
        assert_eq!(r.salience, SalienceSpec::Static(0));
        assert_eq!(r.patterns.len(), 1);
        assert_eq!(r.patterns[0].binding.as_deref(), Some("$p"));
        assert_eq!(r.patterns[0].type_name, "Person");
        assert_eq!(
            r.patterns[0].constraints,
            vec![Constraint::Cmp {
                field: "age".into(),
                op: CmpOp::Gt,
                rhs: CmpRhs::Lit(Literal::I64(18))
            }]
        );
        assert_eq!(
            r.actions,
            vec![Action::Insert {
                type_name: "Adult".into(),
                args: vec![RhsArg::Getter { var: "$p".into(), field: "name".into() }],
            }]
        );
    }

    #[test]
    fn parses_attributes_and_literals() {
        let rules = parse_rules(
            "rule R salience -5 no-loop when Person(name == \"bob\", age <= 3) then insert(new Adult(\"x\")); end",
        )
        .unwrap();
        assert_eq!(rules[0].salience, SalienceSpec::Static(-5));
        assert!(rules[0].no_loop);
        assert_eq!(rules[0].patterns[0].constraints.len(), 2);
    }

    #[test]
    fn parses_phase2_grammar() {
        let rules = parse_rules(
            "rule J when $p : P($a : n, t == false) Q(m > $a) then \
             $p.setT(true); update($p); delete($p); \
             modify($p) { setN(5), setT(false) } end",
        )
        .unwrap();
        let r = &rules[0];
        assert_eq!(r.patterns.len(), 2);
        assert_eq!(
            r.patterns[1].constraints,
            vec![Constraint::Cmp {
                field: "m".into(),
                op: CmpOp::Gt,
                rhs: CmpRhs::Var("$a".into())
            }]
        );
        assert_eq!(r.actions.len(), 6); // set, update, delete, set, set, update
        assert_eq!(
            r.actions[0],
            Action::Set {
                var: "$p".into(),
                field: "t".into(),
                arg: RhsArg::Lit(Literal::Bool(true))
            }
        );
        assert_eq!(r.actions[5], Action::Update { var: "$p".into() });
    }

    #[test]
    fn parses_phase3_operators() {
        let rules = parse_rules(
            "rule O when P(s matches \"a.*\", s contains \"ab\", n in (1, -2, 3.5), m not in (\"x\")) then end",
        )
        .unwrap();
        assert_eq!(
            rules[0].patterns[0].constraints,
            vec![
                Constraint::Matches { field: "s".into(), regex: "a.*".into() },
                Constraint::Contains { field: "s".into(), needle: "ab".into() },
                Constraint::InList {
                    field: "n".into(),
                    items: vec![Literal::I64(1), Literal::I64(-2), Literal::F64(3.5)],
                    negated: false
                },
                Constraint::InList {
                    field: "m".into(),
                    items: vec![Literal::Str("x".into())],
                    negated: true
                },
            ]
        );
        assert!(rules[0].actions.is_empty());
    }

    #[test]
    fn parses_ce_patterns() {
        let rules = parse_rules(
            "rule R when $a : A($x : n) not B(m == $x) exists C(k > 1) then end",
        )
        .unwrap();
        let pats = &rules[0].patterns;
        assert_eq!(pats.len(), 3);
        assert_eq!(pats[0].ce, CeKind::Positive);
        assert_eq!(pats[1].ce, CeKind::Not);
        assert_eq!(pats[1].type_name, "B");
        assert_eq!(pats[2].ce, CeKind::Exists);
        // bindings inside CE patterns are rejected (D-031)
        assert!(parse_rules("rule R when A() not B($x : n) then end").is_err());
        assert!(parse_rules("rule R when A() exists $b : B() then end").is_err());
    }

    #[test]
    fn rejects_out_of_subset() {
        assert!(parse_rules("rule R when accumulate(Person(), $n: count()) then end").is_err());
        // in-lists, matches and contains take literals only (D-030 wall)
        assert!(parse_rules("rule R when P(n in ($a, 2)) then end").is_err());
        assert!(parse_rules("rule R when P(s matches $a) then end").is_err());
        assert!(parse_rules("rule R when P(n in ()) then end").is_err());
    }
}
