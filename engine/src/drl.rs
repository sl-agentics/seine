//! Parser for the supported DRL subset.
//!
//! Grammar (grows strictly by phase; anything outside is a parse error so the
//! scope wall is enforced mechanically):
//!
//! ```text
//! file       := (rule | query)*
//! rule       := "rule" name attr* "when" pattern* "then" action* "end"
//! query      := "query" name [ "(" qparam ("," qparam)* ")" ] qbody "end"
//!               (queries take plain positive patterns only — D-051)
//! qbody      := qbranch ("or" qbranch)*            (D-054)
//! qbranch    := qelem+ | "(" qelem ("and"? qelem)* ")"
//! qelem      := [ "$id" ":" ] IDENT "(" qargs | [constraint ("," constraint)*] ")"
//!               (name resolves to a TYPE (fact pattern) or a QUERY (call)
//!                at compile time; calls are positional-only)
//! qargs      := qarg ("," qarg)* ";"                (positional form)
//! qarg       := "$id" | literal
//! qparam     := ("long"|"double"|"String"|"boolean") "$id"
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
    /// Inline boolean constraint group (D-073): `a == 1 || a == 2`,
    /// `!(x > 5)`, nested parens. Top-level `&&` never appears here —
    /// it splits into separate comma-equivalent constraints at parse
    /// time (ib24/ib28: conjuncts join eq-hash groups and share like
    /// comma constraints). Composites behave like `in` (double
    /// promotion, no hash participation — ib21/ib22/ib23).
    Group(CExpr),
}

/// One leaf/branch of an inline constraint group (D-073).
#[derive(Debug, Clone, PartialEq)]
pub enum CExpr {
    Cmp { field: String, op: CmpOp, rhs: CmpRhs },
    Matches { field: String, regex: String },
    Contains { field: String, needle: String },
    InList { field: String, items: Vec<Literal>, negated: bool },
    And(Vec<CExpr>),
    Or(Vec<CExpr>),
    Not(Box<CExpr>),
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
    /// Some(_) makes this a `?name(args;)` pull query CE (D-056):
    /// type_name holds the QUERY name, constraints/acc are empty.
    pub q_args: Option<Vec<QArg>>,
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
    /// `insertLogical(new T(...));` (D-076): TMS-justified insert — the
    /// fact auto-retracts when its last justification unmatches.
    InsertLogical { type_name: String, args: Vec<RhsArg> },
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
    /// Position in the DRL unit's interleaved rule+query sequence,
    /// counted in TERMINALS: an `or` rule contributes one position per
    /// subrule (D-070) — query agenda items order by (salience 0, this)
    /// (D-058) and rule items by (salience, this).
    pub decl_pos: usize,
    /// Parse-unit id of the source `rule` block. Subrules of one `or`
    /// rule share it: no-loop blocks per PARENT rule (D-070/or_a20).
    pub parent: usize,
}

/// Rule-LHS conditional-element tree (D-070). `or` rewrites to subrules
/// at parse time: the LHS expands to DNF, one pattern list per branch.
#[derive(Debug, Clone, PartialEq)]
enum CeNode {
    Pat(Pattern),
    And(Vec<CeNode>),
    Or(Vec<CeNode>),
}

/// One positional argument of a positional pattern or query call (D-054).
#[derive(Debug, Clone, PartialEq)]
pub enum QArg {
    Var(String),
    Lit(Literal),
}

/// One element of a query branch: a fact pattern (named-constraint or
/// positional form) or a query call — resolved by NAME at compile time.
#[derive(Debug, Clone, PartialEq)]
pub struct QElem {
    pub binding: Option<String>,
    pub name: String,
    pub body: QElemBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QElemBody {
    /// `T(f == $x, $b : g)` — Q0 named-constraint form.
    Named(Vec<Constraint>),
    /// `T($x, 5;)` / `q($x, $z;)` — positional form (D-054).
    Positional(Vec<QArg>),
}

/// A DRL query (Phase Q0 D-049, or-branches and calls Phase Q1 D-054):
/// typed parameters + one or more or-branches of positive elements.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryDef {
    pub name: String,
    /// (type token, `$name`) pairs in declaration order.
    pub params: Vec<(String, String)>,
    pub branches: Vec<Vec<QElem>>,
    /// Position in the DRL unit's interleaved rule+query sequence (D-058).
    pub decl_pos: usize,
}

/// A parsed DRL compilation unit: rules and queries in source order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DrlFile {
    pub rules: Vec<RuleDef>,
    pub queries: Vec<QueryDef>,
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
                "&&" => "&&",
                "||" => "||",
                _ => match c {
                    '!' => "!",
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
                    '?' => "?",
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
    fn peek_at(&self, n: usize) -> Option<&Tok> {
        self.toks.get(self.pos + n)
    }

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

    /// Parse one `rule … end` block. Returns ONE RuleDef per or-branch
    /// (subrule) after DNF expansion (D-070) — a plain rule yields one.
    fn rule(&mut self) -> Result<Vec<RuleDef>, DrlError> {
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
                } else if matches!(self.peek(), Some(Tok::Ident(w)) if w.starts_with('$')) {
                    // bare `salience $v [op $w|lit]` (Drools-legal,
                    // or_a19) — same D-043 expression semantics.
                    let a = self.sal_term()?;
                    let op = match self.peek() {
                        Some(Tok::Sym(o @ ("+" | "-" | "*"))) => {
                            let c = o.chars().next().unwrap();
                            self.next()?;
                            Some((c, self.sal_term()?))
                        }
                        _ => None,
                    };
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
        let mut lhs = Vec::new();
        while !self.at_kw("then") {
            lhs.push(self.lhs_or()?);
        }
        self.expect_kw("then")?;
        let mut actions = Vec::new();
        while !self.at_kw("end") {
            actions.extend(self.actions()?);
        }
        self.expect_kw("end")?;
        // `or` rewrite (D-070): expand the CE tree to DNF, one subrule
        // per branch, in left-major order (or_a23: earlier or-groups
        // vary slowest). Subrules share name/attributes/RHS.
        let branches = expand_ce(&CeNode::And(lhs));
        // Drools' duplicate-declaration rule for FACT bindings (D-070,
        // or_a26/or_b1..b4): within one branch a name may bind once;
        // across or-branches the same name is legal iff it binds the
        // SAME pattern type (field bindings repeat freely, or_a6).
        let mut by_name: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        for b in &branches {
            let mut in_branch = std::collections::HashSet::new();
            for p in b {
                if let Some(v) = &p.binding {
                    if !in_branch.insert(v.as_str())
                        || by_name.insert(v.as_str(), &p.type_name)
                            .is_some_and(|t| t != p.type_name)
                    {
                        return Err(DrlError(format!(
                            "duplicate declaration for variable '{v}' in the rule '{name}' (D-070/or_a26)"
                        )));
                    }
                }
            }
        }
        Ok(branches
            .into_iter()
            .map(|patterns| RuleDef {
                name: name.clone(),
                salience: salience.clone(),
                no_loop,
                patterns,
                actions: actions.clone(),
                decl_pos: 0,
                parent: 0,
            })
            .collect())
    }

    /// `lhsOr := lhsAnd ('or' lhsAnd)*` — infix `or` binds looser than
    /// infix `and`; top-level juxtaposition is AND across whole lhsOr
    /// expressions (or_a4: `A() or B() C()` == `(A or B) and C`).
    fn lhs_or(&mut self) -> Result<CeNode, DrlError> {
        let first = self.lhs_and()?;
        if !self.at_kw("or") {
            return Ok(first);
        }
        let mut branches = vec![first];
        while self.at_kw("or") {
            self.next()?;
            branches.push(self.lhs_and()?);
        }
        Ok(CeNode::Or(branches))
    }

    /// `lhsAnd := lhsUnary ('and' lhsUnary)*`
    fn lhs_and(&mut self) -> Result<CeNode, DrlError> {
        let first = self.lhs_unary()?;
        if !self.at_kw("and") {
            return Ok(first);
        }
        let mut elems = vec![first];
        while self.at_kw("and") {
            self.next()?;
            elems.push(self.lhs_unary()?);
        }
        Ok(CeNode::And(elems))
    }

    /// `lhsUnary := '(' group ')' | pattern`. Inside parens: prefix
    /// `(or …)` / `(and …)` over lhsUnary operands, or ONE infix
    /// expression — bare juxtaposition inside parens is a Drools parse
    /// error (or_a42), mirrored here.
    fn lhs_unary(&mut self) -> Result<CeNode, DrlError> {
        if !matches!(self.peek(), Some(Tok::Sym("("))) {
            return Ok(CeNode::Pat(self.pattern()?));
        }
        self.next()?;
        let node = if self.at_kw("or") {
            self.next()?;
            let mut xs = Vec::new();
            while !matches!(self.peek(), Some(Tok::Sym(")"))) {
                xs.push(self.lhs_unary()?);
            }
            if xs.len() < 2 {
                return Err(DrlError("prefix (or …) needs >= 2 operands".into()));
            }
            CeNode::Or(xs)
        } else if self.at_kw("and") {
            self.next()?;
            let mut xs = Vec::new();
            while !matches!(self.peek(), Some(Tok::Sym(")"))) {
                xs.push(self.lhs_unary()?);
            }
            if xs.len() < 2 {
                return Err(DrlError("prefix (and …) needs >= 2 operands".into()));
            }
            CeNode::And(xs)
        } else {
            self.lhs_or()?
        };
        self.expect_sym(")")?;
        Ok(node)
    }

    /// `query Name(type $p, ...) qbranch (or qbranch)* end` (D-049/D-054);
    /// the engine compiler enforces the D-051/D-055 walls.
    fn query(&mut self) -> Result<QueryDef, DrlError> {
        self.expect_kw("query")?;
        let name = match self.next()? {
            Tok::StrLit(s) => s,
            Tok::Ident(s) => s,
            other => Err(DrlError(format!("expected query name, got {other}")))?,
        };
        let mut params = Vec::new();
        if matches!(self.peek(), Some(Tok::Sym("("))) {
            self.next()?;
            if !matches!(self.peek(), Some(Tok::Sym(")"))) {
                loop {
                    let ty = self.ident()?;
                    let var = self.dollar_ident()?;
                    params.push((ty, var));
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
        }
        let mut branches = vec![self.qbranch()?];
        while self.at_kw("or") {
            self.next()?;
            branches.push(self.qbranch()?);
        }
        self.expect_kw("end")?;
        Ok(QueryDef { name, params, branches, decl_pos: 0 })
    }

    /// One or-branch: a parenthesized `and`-group, or bare elements up to
    /// the next `or`/`end`.
    fn qbranch(&mut self) -> Result<Vec<QElem>, DrlError> {
        let mut elems = Vec::new();
        if matches!(self.peek(), Some(Tok::Sym("("))) {
            self.next()?;
            loop {
                elems.push(self.qelem()?);
                if self.at_kw("and") {
                    self.next()?;
                    continue;
                }
                if matches!(self.peek(), Some(Tok::Sym(")"))) {
                    self.next()?;
                    break;
                }
            }
        } else {
            while !self.at_kw("or") && !self.at_kw("end") {
                elems.push(self.qelem()?);
            }
        }
        if elems.is_empty() {
            return Err(DrlError("empty query branch".into()));
        }
        Ok(elems)
    }

    /// `[$b :] Name( positional-args; | named-constraints )`
    fn qelem(&mut self) -> Result<QElem, DrlError> {
        let first = self.ident()?;
        let (binding, name) = if first.starts_with('$') {
            self.expect_sym(":")?;
            (Some(first), self.ident()?)
        } else {
            (None, first)
        };
        self.expect_sym("(")?;
        // empty parens = named form with no constraints
        if matches!(self.peek(), Some(Tok::Sym(")"))) {
            self.next()?;
            return Ok(QElem { binding, name, body: QElemBody::Named(Vec::new()) });
        }
        // detect positional form: scan for a ';' before the closing paren
        let mut depth = 0usize;
        let mut positional = false;
        for tok in &self.toks[self.pos..] {
            match tok {
                Tok::Sym("(") => depth += 1,
                Tok::Sym(")") => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                Tok::Sym(";") if depth == 0 => {
                    positional = true;
                    break;
                }
                _ => {}
            }
        }
        if positional {
            let mut args = Vec::new();
            loop {
                match self.peek() {
                    Some(Tok::Ident(w)) if w.starts_with('$') => {
                        args.push(QArg::Var(self.ident()?));
                    }
                    _ => args.push(QArg::Lit(self.literal()?)),
                }
                match self.next()? {
                    Tok::Sym(",") => continue,
                    Tok::Sym(";") => break,
                    other => return Err(DrlError(format!("expected ',' or ';', got {other}"))),
                }
            }
            // mixed positional;named tails are out of subset (D-055)
            self.expect_sym(")")?;
            return Ok(QElem { binding, name, body: QElemBody::Positional(args) });
        }
        let mut constraints = Vec::new();
        loop {
            // query bodies keep the pre-D-073 grammar: one plain
            // constraint per comma slot, no inline boolean groups
            // (fence — the query network's sharing/drain semantics for
            // composites are unprobed).
            let slot = self.constraint_slot()?;
            for c in &slot {
                if matches!(c, Constraint::Group(_)) {
                    return Err(DrlError(
                        "inline constraint groups in query bodies are out of subset (D-073)".into(),
                    ));
                }
            }
            constraints.extend(slot);
            match self.next()? {
                Tok::Sym(",") => continue,
                Tok::Sym(")") => break,
                other => return Err(DrlError(format!("expected ',' or ')', got {other}"))),
            }
        }
        Ok(QElem { binding, name, body: QElemBody::Named(constraints) })
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
        if ce != CeKind::Positive && matches!(self.peek(), Some(Tok::Sym("("))) {
            return Err(DrlError(
                "CE groups inside not/exists are out of subset (P1c pending; bare `not T(...)` / `exists T(...)` only, D-031)".into(),
            ));
        }
        if matches!(self.peek(), Some(Tok::Sym("?"))) {
            // `?name(a1, ..., ak;)` pull query CE (D-056/D-057)
            if ce != CeKind::Positive {
                return Err(DrlError(
                    "?query CEs inside not/exists are out of subset (D-057)".into(),
                ));
            }
            self.next()?;
            let name = self.ident()?;
            self.expect_sym("(")?;
            let mut args = Vec::new();
            if matches!(self.peek(), Some(Tok::Sym(")"))) {
                self.next()?;
            } else {
                loop {
                    match self.peek() {
                        Some(Tok::Ident(w)) if w.starts_with('$') => {
                            args.push(QArg::Var(self.ident()?));
                        }
                        _ => args.push(QArg::Lit(self.literal()?)),
                    }
                    match self.next()? {
                        Tok::Sym(",") => continue,
                        Tok::Sym(";") => break,
                        other => {
                            return Err(DrlError(format!("expected ',' or ';', got {other}")))
                        }
                    }
                }
                self.expect_sym(")")?;
            }
            return Ok(Pattern {
                binding: None,
                type_name: name,
                constraints: Vec::new(),
                ce: CeKind::Positive,
                acc: None,
                q_args: Some(args),
            });
        }
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
                constraints.extend(self.constraint_slot()?);
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
                q_args: None,
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
        Ok(Pattern { binding, type_name, constraints, ce, acc: None, q_args: None })
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
            q_args: None,
        })
    }

    /// One comma slot of a pattern (D-073). Yields MULTIPLE constraints
    /// when the slot's top level is `&&` (Drools splits top-level `&&`
    /// into comma-equivalent constraints — they join eq-hash groups and
    /// share alpha nodes exactly like commas, ib24/ib28); `||`/`!()`
    /// tops stay ONE composite Group (in-like semantics, ib21..ib23).
    /// A leading `$v : field` binding may carry a restriction expression
    /// over that field (`$v : b > 0 || b < -5`, ib29/ib12).
    fn constraint_slot(&mut self) -> Result<Vec<Constraint>, DrlError> {
        let mut out = Vec::new();
        let mut cur_field: Option<String> = None;
        if matches!(self.peek(), Some(Tok::Ident(w)) if w.starts_with('$')) {
            let var = self.ident()?;
            self.expect_sym(":")?;
            let field = self.ident()?;
            out.push(Constraint::Bind { var, field: field.clone() });
            // binding with no restriction: slot ends here
            if matches!(self.peek(), Some(Tok::Sym(",")) | Some(Tok::Sym(")"))) {
                return Ok(out);
            }
            cur_field = Some(field);
        }
        let e = self.cexpr_or(&mut cur_field)?;
        match e {
            CExpr::And(xs) => {
                for x in xs {
                    out.push(demote(x));
                }
            }
            other => out.push(demote(other)),
        }
        Ok(out)
    }

    /// `cexpr_or := cexpr_and ('||' cexpr_and)*` — `&&` binds tighter
    /// than `||` (ib5).
    fn cexpr_or(&mut self, cur_field: &mut Option<String>) -> Result<CExpr, DrlError> {
        let first = self.cexpr_and(cur_field)?;
        if !matches!(self.peek(), Some(Tok::Sym("||"))) {
            return Ok(first);
        }
        let mut xs = vec![first];
        while matches!(self.peek(), Some(Tok::Sym("||"))) {
            self.next()?;
            xs.push(self.cexpr_and(cur_field)?);
        }
        Ok(CExpr::Or(xs))
    }

    fn cexpr_and(&mut self, cur_field: &mut Option<String>) -> Result<CExpr, DrlError> {
        let first = self.cexpr_unary(cur_field)?;
        if !matches!(self.peek(), Some(Tok::Sym("&&"))) {
            return Ok(first);
        }
        let mut xs = vec![first];
        while matches!(self.peek(), Some(Tok::Sym("&&"))) {
            self.next()?;
            xs.push(self.cexpr_unary(cur_field)?);
        }
        Ok(CExpr::And(xs))
    }

    /// `cexpr_unary := '!' '(' cexpr_or ')' | '(' cexpr_or ')' | atom`
    fn cexpr_unary(&mut self, cur_field: &mut Option<String>) -> Result<CExpr, DrlError> {
        if matches!(self.peek(), Some(Tok::Sym("!"))) {
            self.next()?;
            self.expect_sym("(")?;
            let inner = self.cexpr_or(cur_field)?;
            self.expect_sym(")")?;
            return Ok(CExpr::Not(Box::new(inner)));
        }
        if matches!(self.peek(), Some(Tok::Sym("("))) {
            self.next()?;
            let inner = self.cexpr_or(cur_field)?;
            self.expect_sym(")")?;
            return Ok(inner);
        }
        self.cexpr_atom(cur_field)
    }

    /// One field test. An atom starting directly with a comparison
    /// operator is the ABBREVIATED form and applies to the most recent
    /// explicitly-named field (`a > 5 && < 10`, ib3/ib4/ib28/ib30);
    /// abbreviated matches/contains/in stay out of subset.
    fn cexpr_atom(&mut self, cur_field: &mut Option<String>) -> Result<CExpr, DrlError> {
        // keyword restriction directly on the current field: the
        // bind-with-restriction forms `$v : f in (…)` / `$v : f matches
        // "…"` (InTest#testInOperator) and abbreviated continuations.
        let kw_restr = match (self.peek(), self.peek_at(1)) {
            (Some(Tok::Ident(w)), Some(Tok::Sym("("))) if w == "in" => true,
            (Some(Tok::Ident(w)), Some(Tok::Ident(w2))) if w == "not" && w2 == "in" => true,
            (Some(Tok::Ident(w)), Some(Tok::StrLit(_)))
                if w == "matches" || w == "contains" =>
            {
                true
            }
            _ => false,
        };
        let field = match self.peek() {
            Some(Tok::Sym("==" | "!=" | "<" | "<=" | ">" | ">=")) => cur_field
                .clone()
                .ok_or_else(|| DrlError("abbreviated restriction with no preceding field".into()))?,
            Some(Tok::Ident(_)) if kw_restr => cur_field
                .clone()
                .ok_or_else(|| DrlError("keyword restriction with no preceding field".into()))?,
            Some(Tok::Ident(w)) if w.starts_with('$') => {
                return Err(DrlError(
                    "bindings inside constraint groups are out of subset (D-073)".into(),
                ))
            }
            _ => {
                let f = self.ident()?;
                *cur_field = Some(f.clone());
                f
            }
        };
        match self.peek() {
            Some(Tok::Ident(w)) if w == "matches" => {
                self.next()?;
                return match self.next()? {
                    Tok::StrLit(s) => Ok(CExpr::Matches { field, regex: s }),
                    other => Err(DrlError(format!(
                        "matches requires a literal string regex, got {other}"
                    ))),
                };
            }
            Some(Tok::Ident(w)) if w == "contains" => {
                self.next()?;
                return match self.next()? {
                    Tok::StrLit(s) => Ok(CExpr::Contains { field, needle: s }),
                    other => Err(DrlError(format!(
                        "contains requires a literal string, got {other}"
                    ))),
                };
            }
            Some(Tok::Ident(w)) if w == "in" => {
                self.next()?;
                let items = self.in_list()?;
                return Ok(CExpr::InList { field, items, negated: false });
            }
            Some(Tok::Ident(w)) if w == "not" => {
                self.next()?;
                self.expect_kw("in")?;
                let items = self.in_list()?;
                return Ok(CExpr::InList { field, items, negated: true });
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
        Ok(CExpr::Cmp { field, op, rhs })
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
            Some(Tok::Ident(w)) if w == "insert" || w == "insertLogical" => {
                let logical = w == "insertLogical";
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
                Ok(vec![if logical {
                    Action::InsertLogical { type_name, args }
                } else {
                    Action::Insert { type_name, args }
                }])
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

/// A split-out slot element (D-073): leaves stay legacy Constraint
/// variants (keeping their eq-hash/sharing identity, ib24); composites
/// become Group.
fn demote(e: CExpr) -> Constraint {
    match e {
        CExpr::Cmp { field, op, rhs } => Constraint::Cmp { field, op, rhs },
        CExpr::Matches { field, regex } => Constraint::Matches { field, regex },
        CExpr::Contains { field, needle } => Constraint::Contains { field, needle },
        CExpr::InList { field, items, negated } => Constraint::InList { field, items, negated },
        other => Constraint::Group(other),
    }
}

/// DNF expansion of a CE tree (D-070): Or concatenates branch lists in
/// listed order (nested or flattens, or_a13x); And crosses factor branch
/// lists left-major (earlier factors vary slowest, or_a23).
fn expand_ce(n: &CeNode) -> Vec<Vec<Pattern>> {
    match n {
        CeNode::Pat(p) => vec![vec![p.clone()]],
        CeNode::Or(xs) => xs.iter().flat_map(expand_ce).collect(),
        CeNode::And(xs) => xs.iter().fold(vec![Vec::new()], |acc, x| {
            let bs = expand_ce(x);
            acc.iter()
                .flat_map(|pre| {
                    bs.iter().map(|b| {
                        let mut v = pre.clone();
                        v.extend(b.iter().cloned());
                        v
                    })
                })
                .collect()
        }),
    }
}

pub fn parse_file(src: &str) -> Result<DrlFile, DrlError> {
    let mut p = Parser { toks: lex(src)?, pos: 0 };
    let mut file = DrlFile::default();
    // decl_pos counts TERMINALS (one per subrule / query, D-070);
    // parent counts source `rule` blocks (no-loop scope).
    let mut pos = 0usize;
    let mut unit = 0usize;
    while p.peek().is_some() {
        if p.at_kw("query") {
            let mut q = p.query()?;
            q.decl_pos = pos;
            pos += 1;
            file.queries.push(q);
        } else {
            for mut r in p.rule()? {
                r.decl_pos = pos;
                r.parent = unit;
                pos += 1;
                file.rules.push(r);
            }
        }
        unit += 1;
    }
    Ok(file)
}

pub fn parse_rules(src: &str) -> Result<Vec<RuleDef>, DrlError> {
    let file = parse_file(src)?;
    if !file.queries.is_empty() {
        return Err(DrlError("queries not expected here (use parse_file)".into()));
    }
    Ok(file.rules)
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
