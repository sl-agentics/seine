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
    /// Exactly 9223372036854775808 (= 2^63): legal only as the operand
    /// of a unary minus, where it folds to i64::MIN — Java folds the
    /// sign into Long.MIN_VALUE literals the same way (JLS 3.10.1).
    IntMinLit,
    FloatLit(f64),
    Sym(&'static str),
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Ident(s) => write!(f, "{s}"),
            Tok::StrLit(s) => write!(f, "{s:?}"),
            Tok::IntLit(n) => write!(f, "{n}"),
            Tok::IntMinLit => write!(f, "9223372036854775808"),
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

/// CEP E2 item E (D-118/D-119): the 13 Allen interval-algebra operators
/// over `@duration` events. Convention `$a:A() $b:B(this <op> $a)` reads
/// "B `op` A" — `this`=B is the SUBJECT, `$a`=A the OBJECT/anchor (the ops
/// are DIRECTIONAL, xdir_* pins). Endpoints: `Xs=X.ts`, `Xe=X.ts+X.dur`.
/// `After`/`Before` are the D-101 temporal-distance ops (mandatory
/// `[lo,hi]`); the other 11 are endpoint relations with optional tolerance
/// params. See `eval_allen` for the full predicate table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllenOp {
    After,
    Before,
    Coincides,
    Meets,
    MetBy,
    Overlaps,
    OverlappedBy,
    During,
    Includes,
    Starts,
    StartedBy,
    Finishes,
    FinishedBy,
}

impl AllenOp {
    /// Parse the operator keyword (following `this`). `None` = not a
    /// temporal operator (the slot is a normal constraint group).
    pub fn from_keyword(w: &str) -> Option<AllenOp> {
        Some(match w {
            "after" => AllenOp::After,
            "before" => AllenOp::Before,
            "coincides" => AllenOp::Coincides,
            "meets" => AllenOp::Meets,
            "metby" => AllenOp::MetBy,
            "overlaps" => AllenOp::Overlaps,
            "overlappedby" => AllenOp::OverlappedBy,
            "during" => AllenOp::During,
            "includes" => AllenOp::Includes,
            "starts" => AllenOp::Starts,
            "startedby" => AllenOp::StartedBy,
            "finishes" => AllenOp::Finishes,
            "finishedby" => AllenOp::FinishedBy,
            _ => return None,
        })
    }

    /// Whether `n` parameters is a valid arity for this op (oracle-pinned,
    /// D-119). `after`/`before` REQUIRE exactly 2 (`[lo,hi]`, byte-identical
    /// to E1); the endpoint ops accept a tolerance/bounds list.
    pub fn arity_ok(self, n: usize) -> bool {
        use AllenOp::*;
        match self {
            After | Before => n == 2,
            // |Bs−As| (start), |Be−Ae| (end): 0 bare, 1 shared dev, 2 split.
            Coincides => n <= 2,
            // single tolerance on the touching endpoint.
            Meets | MetBy | Starts | StartedBy | Finishes | FinishedBy => n <= 1,
            // overlap distance: bare / [max] / [min,max].
            Overlaps | OverlappedBy => n <= 2,
            // start & end distances: bare / [max] / [min,max] / [lo1,hi1,lo2,hi2].
            During | Includes => n == 0 || n == 1 || n == 2 || n == 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    I64(i64),
    F64(f64),
    Str(String),
    Bool(bool),
    /// SQL NULL (D-097). Legal in cmp rhs (only with ==/!= — the
    /// IS [NOT] NULL surface mapping), in in-list members (3VL
    /// membership semantics), and in RHS insert/setter args targeting
    /// nullable fields. Everything else = compile error.
    Null,
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
    /// CEP E1/E2 (D-101/D-118/D-119): `this <op>[params] $a` — an Allen
    /// interval-algebra temporal constraint. `op` = one of the 13 relations;
    /// `params` = 0-4 `duration_ms` values (after/before carry `[lo,hi]`);
    /// `var` = the anchor binding ($a). Reads "B op A" (this=B subject,
    /// $a=A anchor). See `AllenOp` / `eval_allen`.
    Temporal { op: AllenOp, params: Vec<i64>, var: String },
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
    /// D-108: collectList(expr) — ordered value collection (match/
    /// staging order; duplicates kept, one instance leaves per
    /// reverse — ga16).
    CollectList,
    /// D-108: collectSet(expr) — COUNTED value set (a duplicate
    /// survives a sibling's delete — ga15). Iteration order in Drools
    /// is raw HashSet internals (unspecified, D-052-class); both
    /// sides canonicalize SORTED under the SetCollection type.
    CollectSet,
}

/// CEP E2 item B (D-110): a sliding window on an accumulate source.
/// `Time(N)` = `over window:time(N ms)` — an event contributes while
/// `clock − ts < N`, evicted at `ts+N`. `Length(N)` (D-184/D-185) =
/// `over window:length(N)` — a SLOT-RETENTION ring of the last N
/// admissions (post-alpha; corpses keep their slot; eviction pops the
/// oldest slot). N >= 1 (N=0 throws in Drools — out of subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Time(i64),
    Length(i64),
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
    /// D-108 groupby: the KEY binding (a source binding) — Some makes
    /// this a per-group accumulation with one activation per live key.
    pub group_key: Option<String>,
    /// CEP E2 item B (D-110): a sliding window over the source events.
    pub window: Option<Window>,
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
    /// Some(_) makes this a GROUP CE (P1c/D-089): `not(A(…) and B(…))` /
    /// `exists(A and B)` — `ce` holds the outer kind, the inner
    /// patterns are positives (bindings allowed, group-scoped) or bare
    /// not/exists (binding-free, D-031). type_name/constraints/binding
    /// of the group pattern itself are unused. Surface `or` forms are
    /// rewritten away at parse time (LogicTransformer mirror):
    /// `not(A or B)` = `not(A) and not(B)`; `exists(A or B)` =
    /// `not( not(A) and not(B) )`.
    pub group: Option<Vec<Pattern>>,
    /// CEP E2 item D: `Type(...) from entry-point "S1"` draws from a NAMED
    /// entry point (partitioned stream) instead of the DEFAULT working
    /// memory. None = DEFAULT. A pattern only matches facts inserted into
    /// its entry point; the name must be referenced by some rule to be
    /// insertable (Drools: getEntryPoint(unref) is null).
    pub entry_point: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RhsArg {
    Lit(Literal),
    /// `$a` — a field binding from the LHS
    Var(String),
    /// `$p.getName()` — getter on a fact binding; resolved to field `name`
    Getter { var: String, field: String },
}

/// An RHS insert argument expression (D-283 Tier 1): a closed
/// arithmetic grammar — `+ - * / %`, unary minus, parens — over
/// literals, bindings, and getters. Java semantics at evaluation
/// (probes_pending/arith_grammar/PINS.md §A): i64 wraps on overflow,
/// `/` truncates, `%` takes the dividend's sign, division by zero is a
/// runtime error; mixed operands promote to f64 (IEEE). Precedence is
/// STANDARD everywhere — the 9.44 eval-throw on bare `a + b * c` is a
/// self-inconsistent oracle defect we do not copy (D-281).
#[derive(Debug, Clone, PartialEq)]
pub enum RhsExpr {
    Atom(RhsArg),
    Neg(Box<RhsExpr>),
    Bin(char, Box<RhsExpr>, Box<RhsExpr>),
}

impl RhsExpr {
    pub fn is_atom(&self) -> bool {
        matches!(self, RhsExpr::Atom(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Insert { type_name: String, args: Vec<RhsExpr> },
    /// `insertLogical(new T(...));` (D-076): TMS-justified insert — the
    /// fact auto-retracts when its last justification unmatches.
    /// Args stay ATOMS (computed logical args are the stratified tier,
    /// walled at compile until then — D-282).
    InsertLogical { type_name: String, args: Vec<RhsExpr> },
    /// `$p.setX(arg);` — mutates immediately, contributes X to the pending
    /// modification mask consumed by the next `update($p)`.
    Set { var: String, field: String, arg: RhsArg },
    /// `update($p);`
    Update { var: String },
    /// `delete($p);` / `retract($p);`
    Delete { var: String },
    /// `drools.setFocus("g");` (D-106): push the group on the focus
    /// stack (relocating it if already stacked — ag9).
    SetFocus { group: String },
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
    /// `agenda-group "name"` (D-106): None = MAIN.
    pub agenda_group: Option<String>,
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

/// Rule-LHS conditional-element tree (D-070/D-089). `or` rewrites to
/// subrules at parse time: the LHS expands to DNF, one pattern list per
/// branch. Not/Exists wrap a subtree (`not(…)` group forms, P1c) and
/// normalize via the LogicTransformer-mirror rewrites before lowering.
#[derive(Debug, Clone, PartialEq)]
enum CeNode {
    Pat(Pattern),
    And(Vec<CeNode>),
    Or(Vec<CeNode>),
    Not(Box<CeNode>),
    Exists(Box<CeNode>),
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
pub struct DrlError {
    pub msg: String,
    /// Char offset into the source (D-103: positioned errors). Consumed
    /// by attach_position at the parse entry points.
    pub span: Option<u32>,
}

impl fmt::Display for DrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DRL parse error: {}", self.msg)
    }
}

/// Position-less error (post-parse lowering, semantic walls).
fn derr(msg: impl Into<String>) -> DrlError {
    DrlError { msg: msg.into(), span: None }
}

/// Lexer error at char offset `i`.
fn lerr(i: usize, msg: impl Into<String>) -> DrlError {
    DrlError { msg: msg.into(), span: Some(i as u32) }
}

/// D-103: render "line L, col C" + the source line + a caret into the
/// message. Called once at the parse entry points; idempotent on
/// span-less errors.
fn attach_position(mut e: DrlError, src: &str) -> DrlError {
    let Some(span) = e.span.take() else { return e };
    let chars: Vec<char> = src.chars().collect();
    let at = (span as usize).min(chars.len());
    let mut line = 1u32;
    let mut line_start = 0usize;
    for (idx, c) in chars.iter().enumerate().take(at) {
        if *c == '\n' {
            line += 1;
            line_start = idx + 1;
        }
    }
    let col = (at - line_start) as u32 + 1;
    let line_end = chars[line_start..]
        .iter()
        .position(|c| *c == '\n')
        .map(|n| line_start + n)
        .unwrap_or(chars.len());
    let text: String = chars[line_start..line_end].iter().collect();
    let caret = format!("{}^", " ".repeat((col - 1) as usize));
    e.msg = format!("{} at line {line}, col {col}:\n  {text}\n  {caret}", e.msg);
    e
}

fn lex(src: &str) -> Result<(Vec<Tok>, Vec<u32>), DrlError> {
    let b: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    let mut spans: Vec<u32> = Vec::new();
    macro_rules! push {
        ($start:expr, $t:expr) => {{
            spans.push($start as u32);
            out.push($t);
        }};
    }
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
                return Err(lerr(i, "unterminated block comment"));
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
            // `agenda-group` likewise (D-106)
            if word == "agenda" && i + 5 < b.len()
                && b[i..i + 6].iter().collect::<String>() == "-group"
            {
                word = "agenda-group".into();
                i += 6;
            }
            // `entry-point` likewise (CEP E2 item D)
            if word == "entry" && i + 5 < b.len()
                && b[i..i + 6].iter().collect::<String>() == "-point"
            {
                word = "entry-point".into();
                i += 6;
            }
            push!(start, Tok::Ident(word));
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
                push!(start, Tok::FloatLit(s.parse().map_err(|e| {
                    lerr(start, format!("bad float literal {s}: {e}"))
                })?));
            } else {
                let s: String = b[start..i].iter().collect();
                match s.parse::<i64>() {
                    Ok(n) => push!(start, Tok::IntLit(n)),
                    // 2^63 exactly: representable only under a unary
                    // minus (Long.MIN_VALUE); the parser folds or rejects
                    Err(_) if s.parse::<u64>() == Ok(1u64 << 63) => {
                        push!(start, Tok::IntMinLit)
                    }
                    Err(e) => return Err(lerr(start, format!("bad int literal {s}: {e}"))),
                }
            }
        } else if c == '"' {
            let start = i;
            i += 1;
            let mut s = String::new();
            loop {
                if i >= b.len() {
                    return Err(lerr(i, "unterminated string literal"));
                }
                match b[i] {
                    '"' => {
                        i += 1;
                        break;
                    }
                    '\\' => {
                        i += 1;
                        if i >= b.len() {
                            return Err(lerr(i, "unterminated escape"));
                        }
                        s.push(match b[i] {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '"' => '"',
                            other => {
                                return Err(lerr(i, format!("unsupported escape \\{other}")))
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
            push!(start, Tok::StrLit(s));
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
                    // '/' after comment lexing: a lone slash is division
                    // (D-283 RHS arithmetic); '//' stays a comment, as in
                    // Java. '%' is the Java remainder.
                    '/' => "/",
                    '%' => "%",
                    '?' => "?",
                    '[' => "[",
                    ']' => "]",
                    other => return Err(lerr(i, format!("unexpected character {other:?}"))),
                },
            };
            push!(i, Tok::Sym(sym));
            i += sym.len();
        }
    }
    Ok((out, spans))
}

struct Parser {
    toks: Vec<Tok>,
    spans: Vec<u32>,
    pos: usize,
}

impl Parser {
    /// D-103: error at the just-CONSUMED token (post-next() sites —
    /// the "expected X, got {tok}" pattern).
    fn perr_prev(&self, msg: impl Into<String>) -> DrlError {
        let span = self
            .spans
            .get(self.pos.saturating_sub(1))
            .or(self.spans.last())
            .copied();
        DrlError { msg: msg.into(), span }
    }

    /// D-103: error at the CURRENT token's source position (or the
    /// last token's at EOF).
    fn perr(&self, msg: impl Into<String>) -> DrlError {
        let span = self
            .spans
            .get(self.pos)
            .or(self.spans.last())
            .copied();
        DrlError { msg: msg.into(), span }
    }

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
            .ok_or_else(|| self.perr("unexpected end of input"))?;
        self.pos += 1;
        Ok(t)
    }

    fn expect_sym(&mut self, s: &str) -> Result<(), DrlError> {
        match self.next()? {
            Tok::Sym(x) if x == s => Ok(()),
            other => Err(self.perr_prev(format!("expected {s:?}, got {other}"))),
        }
    }

    fn expect_kw(&mut self, kw: &str) -> Result<(), DrlError> {
        match self.next()? {
            Tok::Ident(x) if x == kw => Ok(()),
            other => Err(self.perr_prev(format!("expected keyword {kw:?}, got {other}"))),
        }
    }

    fn at_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(x)) if x == kw)
    }

    fn ident(&mut self) -> Result<String, DrlError> {
        match self.next()? {
            Tok::Ident(x) => Ok(x),
            other => Err(self.perr_prev(format!("expected identifier, got {other}"))),
        }
    }

    /// `100ms` / `2s` — lexed as IntLit + unit Ident (CEP E1).
    fn duration_ms(&mut self) -> Result<i64, DrlError> {
        let n = match self.next()? {
            Tok::IntLit(n) => n,
            other => return Err(self.perr_prev(format!("expected duration, got {other}"))),
        };
        match self.next()? {
            Tok::Ident(u) if u == "ms" => Ok(n),
            Tok::Ident(u) if u == "s" => Ok(n * 1000),
            other => Err(self.perr_prev(format!("expected ms/s unit, got {other}"))),
        }
    }

    /// CEP E2 item B (D-110): optional `over window:time(N ms)` after an
    /// accumulate source pattern. `window:length` is walled to a
    /// follow-on slab; the standalone-pattern window form is unsupported
    /// (natural parse wall at `over`).
    fn parse_window_opt(&mut self) -> Result<Option<Window>, DrlError> {
        if !self.at_kw("over") {
            return Ok(None);
        }
        self.expect_kw("over")?;
        self.expect_kw("window")?;
        self.expect_sym(":")?;
        let kind = self.ident()?;
        self.expect_sym("(")?;
        let w = match kind.as_str() {
            "time" => Window::Time(self.duration_ms()?),
            "length" => {
                // D-184/D-185: the wall lifts for accumulate sources (the
                // only place this parser is reached from).
                let n = match self.next()? {
                    Tok::IntLit(n) => n,
                    other => {
                        return Err(
                            self.perr_prev(format!("expected window length, got {other}"))
                        )
                    }
                };
                if n < 1 {
                    return Err(self.perr(
                        "window:length(N) requires N >= 1 (N=0 throws in Drools; D-184)",
                    ));
                }
                Window::Length(n)
            }
            other => {
                return Err(self.perr(format!("unknown window kind {other:?} (window:time only)")))
            }
        };
        self.expect_sym(")")?;
        Ok(Some(w))
    }

    fn literal(&mut self) -> Result<Literal, DrlError> {
        match self.next()? {
            Tok::IntLit(n) => Ok(Literal::I64(n)),
            Tok::IntMinLit => Err(self.perr_prev(
                "integer number too large: 9223372036854775808 (only \
                 -9223372036854775808 is representable)",
            )),
            Tok::FloatLit(n) => Ok(Literal::F64(n)),
            Tok::StrLit(s) => Ok(Literal::Str(s)),
            Tok::Ident(w) if w == "true" => Ok(Literal::Bool(true)),
            Tok::Ident(w) if w == "false" => Ok(Literal::Bool(false)),
            Tok::Ident(w) if w == "null" => Ok(Literal::Null),
            Tok::Sym("-") => match self.next()? {
                Tok::IntLit(n) => Ok(Literal::I64(-n)),
                Tok::IntMinLit => Ok(Literal::I64(i64::MIN)),
                Tok::FloatLit(n) => Ok(Literal::F64(-n)),
                other => Err(self.perr_prev(format!("expected number after '-', got {other}"))),
            },
            other => Err(self.perr_prev(format!("expected literal, got {other}"))),
        }
    }

    /// Parse one `rule … end` block. Returns ONE RuleDef per or-branch
    /// (subrule) after DNF expansion (D-070) — a plain rule yields one.
    fn rule(&mut self) -> Result<Vec<RuleDef>, DrlError> {
        self.expect_kw("rule")?;
        let name = match self.next()? {
            Tok::StrLit(s) => s,
            Tok::Ident(s) => s,
            other => Err(self.perr_prev(format!("expected rule name, got {other}")))?,
        };
        let mut salience = SalienceSpec::Static(0);
        let mut no_loop = false;
        let mut agenda_group: Option<String> = None;
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
                            return Err(self.perr(format!(
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
            } else if self.at_kw("agenda-group") {
                self.next()?;
                match self.next()? {
                    Tok::StrLit(s) => agenda_group = Some(s),
                    other => {
                        return Err(self.perr_prev(format!(
                            "agenda-group takes a string name, got {other:?}"
                        )))
                    }
                }
            } else if self.at_kw("when") {
                self.next()?;
                break;
            } else {
                return Err(self.perr(format!(
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
        // `or` rewrite (D-070) + group normalization (D-089): the
        // LogicTransformer-mirror pass rewrites `or` out from under
        // not/exists, then the tree expands to DNF, one subrule per
        // branch, in left-major order (or_a23: earlier or-groups vary
        // slowest). Subrules share name/attributes/RHS.
        let tree = normalize_ce(CeNode::And(lhs))?;
        let branches = expand_ce(&tree)?;
        // Drools' duplicate-declaration rule for FACT bindings (D-070,
        // or_a26/or_b1..b4): within one branch a name may bind once;
        // across or-branches the same name is legal iff it binds the
        // SAME pattern type (field bindings repeat freely, or_a6).
        // Group-inner fact bindings join the check (conservative: inner
        // names may not shadow outer ones — resolution stays
        // unambiguous; the subset is stricter than Drools here).
        let mut by_name: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for b in &branches {
            let mut in_branch = std::collections::HashSet::new();
            let all = b.iter().flat_map(|p| {
                std::iter::once(p).chain(p.group.iter().flatten())
            });
            for p in all {
                if let Some(v) = &p.binding {
                    if !in_branch.insert(v.clone())
                        || by_name
                            .insert(v.clone(), p.type_name.clone())
                            .is_some_and(|t| t != p.type_name)
                    {
                        return Err(self.perr(format!(
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
                agenda_group: agenda_group.clone(),
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
        // `not ( … )` / `exists ( … )` GROUP forms (P1c/D-089): the
        // keyword directly followed by '(' wraps a CE subtree. Bare
        // `not T(...)` (Ident after the keyword) stays on the pattern
        // path. A single-pattern group `not (A())` collapses to the
        // bare CE at lowering (the or_a41 fence lift).
        if (self.at_kw("not") || self.at_kw("exists"))
            && matches!(self.toks.get(self.pos + 1), Some(Tok::Sym("(")))
        {
            let is_not = self.at_kw("not");
            self.next()?;
            self.expect_sym("(")?;
            let inner = self.lhs_or()?;
            self.expect_sym(")")?;
            return Ok(if is_not {
                CeNode::Not(Box::new(inner))
            } else {
                CeNode::Exists(Box::new(inner))
            });
        }
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
                return Err(self.perr("prefix (or …) needs >= 2 operands"));
            }
            CeNode::Or(xs)
        } else if self.at_kw("and") {
            self.next()?;
            let mut xs = Vec::new();
            while !matches!(self.peek(), Some(Tok::Sym(")"))) {
                xs.push(self.lhs_unary()?);
            }
            if xs.len() < 2 {
                return Err(self.perr("prefix (and …) needs >= 2 operands"));
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
            other => Err(self.perr_prev(format!("expected query name, got {other}")))?,
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
                            return Err(self.perr_prev(format!("expected ',' or ')', got {other}")))
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
            return Err(self.perr("empty query branch"));
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
                    other => return Err(self.perr_prev(format!("expected ',' or ';', got {other}"))),
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
                    return Err(self.perr("inline constraint groups in query bodies are out of subset (D-073)"));
                }
            }
            constraints.extend(slot);
            match self.next()? {
                Tok::Sym(",") => continue,
                Tok::Sym(")") => break,
                other => return Err(self.perr_prev(format!("expected ',' or ')', got {other}"))),
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
                other => Err(self.perr(format!(
                    "salience terms are int literals or bindings, got {other:?}"
                ))),
            },
            Some(Tok::Ident(w)) if w.starts_with('$') => Ok(SalTerm::Var(self.dollar_ident()?)),
            other => Err(self.perr(format!(
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
            return Err(self.perr("CE groups inside not/exists are out of subset (P1c pending; bare `not T(...)` / `exists T(...)` only, D-031)"));
        }
        if matches!(self.peek(), Some(Tok::Sym("?"))) {
            // `?name(a1, ..., ak;)` pull query CE (D-056/D-057)
            if ce != CeKind::Positive {
                return Err(self.perr("?query CEs inside not/exists are out of subset (D-057)"));
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
                            return Err(self.perr_prev(format!("expected ',' or ';', got {other}")))
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
                group: None,
                entry_point: None,
            });
        }
        if self.at_kw("accumulate") {
            if ce != CeKind::Positive {
                return Err(self.perr("not/exists over accumulate not in subset"));
            }
            return self.accumulate_pattern();
        }
        if self.at_kw("groupby") {
            if ce != CeKind::Positive {
                return Err(self.perr("not/exists over groupby not in subset"));
            }
            return self.groupby_pattern();
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
                        return Err(self.perr_prev(format!("expected ',' or ')', got {other}")))
                    }
                }
            }
        } else {
            self.next()?;
        }
        let mut entry_point: Option<String> = None;
        if self.at_kw("from") {
            self.next()?;
            if self.at_kw("entry-point") {
                // CEP E2 item D: `Type(...) from entry-point "S1"` — a NAMED
                // stream source. Falls through to the normal pattern return
                // carrying entry_point; source membership is EP-filtered at
                // routing time (alpha_passes).
                self.next()?;
                entry_point = Some(match self.next()? {
                    Tok::StrLit(s) => s,
                    other => {
                        return Err(self.perr_prev(format!(
                            "entry-point name must be a string literal, got {other}"
                        )))
                    }
                });
            } else if self.at_kw("collect") {
                self.next()?;
                if ce != CeKind::Positive {
                    return Err(self.perr("not/exists over collect not in subset"));
                }
                if !matches!(type_name.as_str(), "List" | "ArrayList" | "Collection") {
                    return Err(self.perr(format!(
                        "collect result pattern must be List/ArrayList/Collection, got {type_name}"
                    )));
                }
                if !constraints.is_empty() {
                    return Err(self.perr("constraints on the collect result pattern are not in subset"));
                }
                let result_var = binding
                    .ok_or_else(|| self.perr("collect result must be bound (`$l : List()`)"))?;
                self.expect_sym("(")?;
                let src = self.pattern()?;
                self.expect_sym(")")?;
                if src.ce != CeKind::Positive || src.acc.is_some() {
                    return Err(self.perr("collect source must be a plain pattern"));
                }
                if src.binding.is_some()
                    || src.constraints.iter().any(|c| matches!(c, Constraint::Bind { .. }))
                {
                    return Err(self.perr("bindings inside a collect source are not in subset"));
                }
                // A collect source referencing outer bindings builds an RIA
                // SUBNETWORK — unported territory with its own quirks
                // (D-041/fz_999_4371): alpha-only sources stay in subset.
                if src.constraints.iter().any(
                    |c| matches!(c, Constraint::Cmp { rhs: CmpRhs::Var(_), .. }),
                ) {
                    return Err(self.perr("variable references inside a collect source are not in subset (subnetwork, D-041)"));
                }
                return Ok(Pattern {
                    binding: None,
                    type_name: src.type_name,
                    constraints: src.constraints,
                    ce: CeKind::Positive,
                    acc: Some(AccSpec { func: AccFunc::Collect, arg: None, result_var , group_key: None, window: None }),
                    q_args: None,
                    group: None,
                    entry_point: None,
                });
            } else {
                return Err(self.perr("`from` is only supported as `from collect` or `from entry-point \"…\"` (D-038, item D)"));
            }
        }
        if ce != CeKind::Positive {
            // Bindings inside not/exists are scoped out in Drools; the
            // subset rejects them outright (D-031).
            if binding.is_some()
                || constraints.iter().any(|c| matches!(c, Constraint::Bind { .. }))
            {
                return Err(self.perr("bindings are not allowed in not/exists patterns"));
            }
        }
        Ok(Pattern { binding, type_name, constraints, ce, acc: None, q_args: None, group: None, entry_point })
    }

    /// `accumulate( <source pattern> ; $r : func([$arg]) )` — built-in
    /// functions only; multi-function and custom (init/action/result)
    /// accumulates are out of subset (D-038).
    /// D-108: `groupby( SOURCE ; $key ; $res : func($arg) )` — one
    /// activation per live key (ga3/ga8/ga9/ga10 pins).
    fn groupby_pattern(&mut self) -> Result<Pattern, DrlError> {
        self.expect_kw("groupby")?;
        self.expect_sym("(")?;
        let src = self.pattern()?;
        if src.ce != CeKind::Positive || src.acc.is_some() || src.binding.is_some() {
            return Err(self.perr("groupby source must be a plain unbound pattern"));
        }
        self.expect_sym(";")?;
        let key_var = self.dollar_ident()?;
        if !src
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::Bind { var, .. } if var == &key_var))
        {
            return Err(self.perr(format!(
                "groupby key {key_var} must be a binding declared in the source pattern"
            )));
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
            "collectList" => AccFunc::CollectList,
            "collectSet" => AccFunc::CollectSet,
            other => {
                return Err(self.perr(format!(
                    "groupby function {other:?} not in subset"
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
        self.expect_sym(")")?;
        if func != AccFunc::Count && arg.is_none() {
            return Err(self.perr(format!("{fname} requires a bound argument")));
        }
        if let Some(a) = &arg {
            if !src
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::Bind { var, .. } if var == a))
            {
                return Err(self.perr(format!(
                    "groupby argument {a} must be a binding declared in the source pattern"
                )));
            }
        }
        Ok(Pattern {
            acc: Some(AccSpec { func, arg, result_var, group_key: Some(key_var), window: None }),
            ..src
        })
    }

    fn accumulate_pattern(&mut self) -> Result<Pattern, DrlError> {
        self.expect_kw("accumulate")?;
        self.expect_sym("(")?;
        let mut src = self.pattern()?;
        if src.ce != CeKind::Positive || src.acc.is_some() || src.binding.is_some() {
            return Err(self.perr("accumulate source must be a plain unbound pattern"));
        }
        let window = self.parse_window_opt()?;
        // CEP E2 item D: a WINDOWED source's entry-point trails the window
        // (`E() over window:time(N) from entry-point "S1"`); the no-window
        // case parsed it in pattern() already.
        if self.at_kw("from") {
            self.next()?;
            self.expect_kw("entry-point")?;
            src.entry_point = Some(match self.next()? {
                Tok::StrLit(s) => s,
                other => {
                    return Err(self.perr_prev(format!(
                        "entry-point name must be a string literal, got {other}"
                    )))
                }
            });
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
            "collectList" => AccFunc::CollectList,
            "collectSet" => AccFunc::CollectSet,
            other => {
                return Err(self.perr(format!(
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
            return Err(self.perr("multi-function accumulate not in subset"));
        }
        self.expect_sym(")")?;
        if func != AccFunc::Count && arg.is_none() {
            return Err(self.perr(format!("{fname} requires a bound argument")));
        }
        // source bindings are scoped inside the accumulate; the arg must
        // be one of them (unused extras are legal and simply ignored)
        if let Some(a) = &arg {
            if !src
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::Bind { var, .. } if var == a))
            {
                return Err(self.perr(format!("unknown accumulate argument {a}")));
            }
        }
        Ok(Pattern {
            binding: None,
            type_name: src.type_name,
            constraints: src.constraints,
            ce: CeKind::Positive,
            acc: Some(AccSpec { func, arg, result_var, group_key: None, window }),
            q_args: None,
            group: None,
            entry_point: src.entry_point,
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
        // CEP E1/E2: `this <op>[params] $x` — a whole-slot temporal
        // constraint (no composition with groups). `op` is one of the 13
        // Allen relations (D-119); `after`/`before` (D-101) mandate `[lo,hi]`,
        // the endpoint ops take an optional 0-4 tolerance list.
        if matches!(self.peek(), Some(Tok::Ident(w)) if w == "this") {
            self.next()?;
            let opw = self.ident()?;
            let op = AllenOp::from_keyword(&opw).ok_or_else(|| {
                self.perr(format!("expected a temporal operator following 'this', got {opw}"))
            })?;
            let mut params = Vec::new();
            if matches!(self.peek(), Some(Tok::Sym("["))) {
                self.expect_sym("[")?;
                // non-empty bracket: comma-separated `duration_ms` values.
                loop {
                    params.push(self.duration_ms()?);
                    if matches!(self.peek(), Some(Tok::Sym(","))) {
                        self.next()?;
                        continue;
                    }
                    break;
                }
                self.expect_sym("]")?;
            }
            if !op.arity_ok(params.len()) {
                return Err(self.perr(format!(
                    "operator {opw} does not accept {} parameter(s)",
                    params.len()
                )));
            }
            let var = self.dollar_ident()?;
            out.push(Constraint::Temporal { op, params, var });
            return Ok(out);
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
                .ok_or_else(|| self.perr("abbreviated restriction with no preceding field"))?,
            Some(Tok::Ident(_)) if kw_restr => cur_field
                .clone()
                .ok_or_else(|| self.perr("keyword restriction with no preceding field"))?,
            Some(Tok::Ident(w)) if w.starts_with('$') => {
                return Err(self.perr("bindings inside constraint groups are out of subset (D-073)"))
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
                    other => Err(self.perr(format!(
                        "matches requires a literal string regex, got {other}"
                    ))),
                };
            }
            Some(Tok::Ident(w)) if w == "contains" => {
                self.next()?;
                return match self.next()? {
                    Tok::StrLit(s) => Ok(CExpr::Contains { field, needle: s }),
                    other => Err(self.perr(format!(
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
            other => return Err(self.perr_prev(format!("expected comparison operator, got {other}"))),
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
                other => return Err(self.perr_prev(format!("expected ',' or ')', got {other}"))),
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
                        args.push(self.rhs_expr()?);
                        match self.next()? {
                            Tok::Sym(",") => continue,
                            Tok::Sym(")") => break,
                            other => {
                                return Err(self.perr_prev(format!("expected ',' or ')', got {other}")))
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
                                return Err(self.perr_prev(format!("expected ',' or '}}', got {other}")))
                            }
                        }
                    }
                } else {
                    self.next()?;
                }
                out.push(Action::Update { var });
                Ok(out)
            }
            Some(Tok::Ident(w)) if w == "drools" => {
                self.next()?;
                self.expect_sym(".")?;
                let meth = self.ident()?;
                if meth != "setFocus" {
                    return Err(self.perr_prev(format!(
                        "drools.{meth}: only setFocus is in the certified subset (D-106)"
                    )));
                }
                self.expect_sym("(")?;
                let group = match self.next()? {
                    Tok::StrLit(s) => s,
                    other => {
                        return Err(self.perr_prev(format!(
                            "setFocus takes a string group name, got {other:?}"
                        )))
                    }
                };
                self.expect_sym(")")?;
                self.expect_sym(";")?;
                Ok(vec![Action::SetFocus { group }])
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
            other => Err(self.perr(format!(
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
            Err(self.perr_prev(format!("expected $binding, got {id}")))
        }
    }

    /// Insert-arg expression: additive over multiplicative over factor.
    /// Standard precedence, left-associative (D-281: we do not copy the
    /// oracle's bare `a + b * c` eval defect — witnesses in xfail/).
    fn rhs_expr(&mut self) -> Result<RhsExpr, DrlError> {
        let mut e = self.rhs_term()?;
        loop {
            match self.peek() {
                Some(Tok::Sym(s @ ("+" | "-"))) => {
                    let op = s.chars().next().unwrap();
                    self.next()?;
                    e = RhsExpr::Bin(op, Box::new(e), Box::new(self.rhs_term()?));
                }
                _ => return Ok(e),
            }
        }
    }

    fn rhs_term(&mut self) -> Result<RhsExpr, DrlError> {
        let mut e = self.rhs_factor()?;
        loop {
            match self.peek() {
                Some(Tok::Sym(s @ ("*" | "/" | "%"))) => {
                    let op = s.chars().next().unwrap();
                    self.next()?;
                    e = RhsExpr::Bin(op, Box::new(e), Box::new(self.rhs_factor()?));
                }
                _ => return Ok(e),
            }
        }
    }

    fn rhs_factor(&mut self) -> Result<RhsExpr, DrlError> {
        match self.peek() {
            Some(Tok::Sym("(")) => {
                self.next()?;
                let e = self.rhs_expr()?;
                self.expect_sym(")")?;
                Ok(e)
            }
            Some(Tok::Sym("-")) => {
                // `-3` stays a signed literal (the literal() path owns the
                // IntMinLit magnitude case); `-$a` / `-( … )` is negation.
                if matches!(
                    self.peek_at(1),
                    Some(Tok::IntLit(_) | Tok::FloatLit(_) | Tok::IntMinLit)
                ) {
                    Ok(RhsExpr::Atom(self.rhs_arg()?))
                } else {
                    self.next()?;
                    Ok(RhsExpr::Neg(Box::new(self.rhs_factor()?)))
                }
            }
            _ => Ok(RhsExpr::Atom(self.rhs_arg()?)),
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
                            self.perr(format!("unsupported method call .{getter}() (only getters)"))
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
        .ok_or_else(|| derr(format!("expected setter, got {setter}")))
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

/// LogicTransformer-mirror normalization (D-089, pinned by sn_f1/f2/f4):
/// bottom-up, `or` is rewritten out from under not/exists —
/// `not(A or B)` → `and(not A, not B)` (NotOrTransformation);
/// `exists(A or B)` → `not( and( not(A), not(B) ) )`
/// (ExistOrTransformation); an `and` child holding an `or` distributes
/// first (AndOr pull-up, left-major like or_a23). Single-child groups
/// pack. After normalize, Or appears only ABOVE all Not/Exists nodes.
fn normalize_ce(n: CeNode) -> Result<CeNode, DrlError> {
    fn pack(mut xs: Vec<CeNode>, or: bool) -> CeNode {
        if xs.len() == 1 {
            return xs.pop().unwrap();
        }
        // flatten same-type nesting (GroupElement.pack)
        let mut flat = Vec::new();
        for x in xs {
            match (or, x) {
                (true, CeNode::Or(inner)) => flat.extend(inner),
                (false, CeNode::And(inner)) => flat.extend(inner),
                (_, other) => flat.push(other),
            }
        }
        if or { CeNode::Or(flat) } else { CeNode::And(flat) }
    }
    /// Distribute And-over-Or (left-major) so a not/exists child with a
    /// buried `or` becomes a top Or of and-branches.
    fn pull_or_up(n: CeNode) -> CeNode {
        match n {
            CeNode::And(xs) if xs.iter().any(|x| matches!(x, CeNode::Or(_))) => {
                let branches = xs.iter().fold(vec![Vec::new()], |acc: Vec<Vec<CeNode>>, x| {
                    let bs: Vec<CeNode> = match x {
                        CeNode::Or(alts) => alts.clone(),
                        other => vec![other.clone()],
                    };
                    acc.iter()
                        .flat_map(|pre| {
                            bs.iter().map(|b| {
                                let mut v = pre.clone();
                                v.push(b.clone());
                                v
                            })
                        })
                        .collect()
                });
                CeNode::Or(branches.into_iter().map(|b| pack(b, false)).collect())
            }
            other => other,
        }
    }
    Ok(match n {
        CeNode::Pat(_) => n,
        CeNode::And(xs) => {
            let xs = xs.into_iter().map(normalize_ce).collect::<Result<Vec<_>, _>>()?;
            pack(xs, false)
        }
        CeNode::Or(xs) => {
            let xs = xs.into_iter().map(normalize_ce).collect::<Result<Vec<_>, _>>()?;
            pack(xs, true)
        }
        CeNode::Not(x) => {
            let x = pull_or_up(normalize_ce(*x)?);
            match x {
                CeNode::Or(bs) => normalize_ce(CeNode::And(
                    bs.into_iter().map(|b| CeNode::Not(Box::new(b))).collect(),
                ))?,
                other => CeNode::Not(Box::new(other)),
            }
        }
        CeNode::Exists(x) => {
            let x = pull_or_up(normalize_ce(*x)?);
            match x {
                CeNode::Or(bs) => normalize_ce(CeNode::Not(Box::new(CeNode::And(
                    bs.into_iter().map(|b| CeNode::Not(Box::new(b))).collect(),
                ))))?,
                other => CeNode::Exists(Box::new(other)),
            }
        }
    })
}

/// Lower a normalized not/exists child into ONE pattern: a bare CE
/// (single plain inner pattern — the or_a41 collapse) or a GROUP CE
/// (P1c/D-089). Fences: composite groups nested inside groups
/// (RIA-in-RIA), acc/collect/?query inside groups, bindings on bare-CE
/// members (D-031), inner element count 2..=3.
fn lower_group(kind: CeKind, child: &CeNode) -> Result<Pattern, DrlError> {
    fn leaf(elem: &CeNode) -> Result<Pattern, DrlError> {
        match elem {
            CeNode::Pat(p) => {
                if p.acc.is_some() {
                    return Err(derr("accumulate/collect inside not/exists groups is out of subset (D-089)"));
                }
                if p.q_args.is_some() {
                    return Err(derr("?query CEs inside not/exists are out of subset (D-057)"));
                }
                Ok(p.clone())
            }
            CeNode::Not(x) | CeNode::Exists(x) => {
                let inner_kind = if matches!(elem, CeNode::Not(_)) {
                    CeKind::Not
                } else {
                    CeKind::Exists
                };
                match x.as_ref() {
                    CeNode::Pat(p) if p.ce == CeKind::Positive => {
                        let mut p = leaf(&CeNode::Pat(p.clone()))?;
                        if p.binding.is_some()
                            || p.constraints.iter().any(|c| matches!(c, Constraint::Bind { .. }))
                        {
                            return Err(derr("bindings are not allowed in not/exists patterns"));
                        }
                        p.ce = inner_kind;
                        Ok(p)
                    }
                    _ => Err(derr("composite groups nested inside not/exists are out of subset \
                         (RIA-in-RIA — e.g. not(exists(A and B)), not(not(A)); D-089)")),
                }
            }
            _ => Err(derr("composite groups nested inside not/exists are out of subset \
                 (RIA-in-RIA — e.g. not(exists(A and B)), not(not(A)); D-089)")),
        }
    }
    match child {
        // single plain inner pattern: collapse to the bare CE (or_a41)
        CeNode::Pat(p0) if p0.ce == CeKind::Positive && p0.group.is_none() => {
            let mut p = leaf(child)?;
            if p.binding.is_some()
                || p.constraints.iter().any(|c| matches!(c, Constraint::Bind { .. }))
            {
                return Err(derr("bindings are not allowed in not/exists patterns"));
            }
            p.ce = kind;
            Ok(p)
        }
        CeNode::And(elems) => {
            if !(2..=3).contains(&elems.len()) {
                return Err(derr(format!(
                    "not/exists groups are limited to 2-3 inner patterns, got {} (D-089)",
                    elems.len()
                )));
            }
            let inner = elems.iter().map(leaf).collect::<Result<Vec<_>, _>>()?;
            Ok(Pattern {
                binding: None,
                type_name: String::new(),
                constraints: Vec::new(),
                ce: kind,
                acc: None,
                q_args: None,
                group: Some(inner),
                entry_point: None,
            })
        }
        _ => Err(derr("composite groups nested inside not/exists are out of subset \
             (RIA-in-RIA — e.g. not(exists(A and B)), not(not(A)); D-089)")),
    }
}

/// DNF expansion of a normalized CE tree (D-070): Or concatenates branch
/// lists in listed order (nested or flattens, or_a13x); And crosses
/// factor branch lists left-major (earlier factors vary slowest,
/// or_a23). Not/Exists nodes lower to single (possibly group) patterns.
fn expand_ce(n: &CeNode) -> Result<Vec<Vec<Pattern>>, DrlError> {
    Ok(match n {
        CeNode::Pat(p) => vec![vec![p.clone()]],
        CeNode::Not(x) => vec![vec![lower_group(CeKind::Not, x)?]],
        CeNode::Exists(x) => vec![vec![lower_group(CeKind::Exists, x)?]],
        CeNode::Or(xs) => {
            let mut out = Vec::new();
            for x in xs {
                out.extend(expand_ce(x)?);
            }
            out
        }
        CeNode::And(xs) => {
            let mut acc: Vec<Vec<Pattern>> = vec![Vec::new()];
            for x in xs {
                let bs = expand_ce(x)?;
                acc = acc
                    .iter()
                    .flat_map(|pre| {
                        bs.iter().map(|b| {
                            let mut v = pre.clone();
                            v.extend(b.iter().cloned());
                            v
                        })
                    })
                    .collect();
            }
            acc
        }
    })
}

pub fn parse_file(src: &str) -> Result<DrlFile, DrlError> {
    parse_file_inner(src).map_err(|e| attach_position(e, src))
}

fn parse_file_inner(src: &str) -> Result<DrlFile, DrlError> {
    let (toks, spans) = lex(src)?;
    let mut p = Parser { toks, spans, pos: 0 };
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
        return Err(derr("queries not expected here (use parse_file)"));
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
                args: vec![RhsExpr::Atom(RhsArg::Getter {
                    var: "$p".into(),
                    field: "name".into()
                })],
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
    fn group_ce_parsing_and_rewrites() {
        // basic group
        let rules =
            parse_rules("rule R when not(A() and B()) then end").unwrap();
        assert_eq!(rules.len(), 1);
        let p = &rules[0].patterns[0];
        assert_eq!(p.ce, CeKind::Not);
        let g = p.group.as_ref().unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].type_name, "A");
        assert_eq!(g[1].type_name, "B");

        // single-pattern group collapses to the bare CE (or_a41 lift)
        let rules = parse_rules("rule R when not (A()) then end").unwrap();
        assert_eq!(rules[0].patterns[0].ce, CeKind::Not);
        assert!(rules[0].patterns[0].group.is_none());

        // De Morgan: not(A or B) == and(not A, not B) — ONE subrule,
        // TWO bare not patterns (sn_f1a)
        let rules =
            parse_rules("rule R when not(A() or B()) then end").unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].patterns.len(), 2);
        assert!(rules[0].patterns.iter().all(|p| p.ce == CeKind::Not && p.group.is_none()));

        // double negation: exists(A or B) == not(and(not A, not B)) —
        // ONE group pattern with two inner bare nots (sn_f2)
        let rules =
            parse_rules("rule R when exists(A() or B()) then end").unwrap();
        assert_eq!(rules.len(), 1);
        let p = &rules[0].patterns[0];
        assert_eq!(p.ce, CeKind::Not);
        let g = p.group.as_ref().unwrap();
        assert_eq!(g.len(), 2);
        assert!(g.iter().all(|ip| ip.ce == CeKind::Not));

        // not(or of ands) distributes into two group conjuncts (sn_f4)
        let rules = parse_rules(
            "rule R when not((A() and B()) or (C() and D())) then end",
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].patterns.len(), 2);
        assert!(rules[0].patterns.iter().all(|p| p.group.is_some()));

        // inner bindings + bare CEs inside groups parse (sn_a6/sn_g5)
        parse_rules("rule R when not(A($y : k) and B(m == $y)) then end").unwrap();
        parse_rules("rule R when not(A() and not(B())) then end").unwrap();
        parse_rules("rule R when not(A() and exists(B())) then end").unwrap();
    }

    #[test]
    fn group_ce_fences() {
        // RIA-in-RIA (D-089)
        assert!(parse_rules("rule R when not(exists(A() and B())) then end").is_err());
        assert!(parse_rules("rule R when not(not(A())) then end").is_err());
        assert!(parse_rules("rule R when exists((A() and B()) or C()) then end").is_err());
        // >3 inner patterns
        assert!(
            parse_rules("rule R when not(A() and B() and C() and D()) then end").is_err()
        );
        // bindings on bare-CE members stay out (D-031)
        assert!(parse_rules("rule R when not(A($x : k) and not(B($y : m))) then end").is_err());
        // accumulate / ?query inside groups
        assert!(parse_rules(
            "rule R when not(accumulate( A($v : k) ; $s : sum($v) ) and B()) then end"
        )
        .is_err());
        assert!(parse_rules("rule R when not(?q($x;) and B()) then end").is_err());
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
