//! Bounded, cooperative execution of a deliberately explicit subset of VBA.
//!
//! Unsupported syntax is an error, rather than an ignored statement. Object
//! identity and the Excel object model belong to the host, not to this module.

use std::collections::BTreeMap;
use std::fmt;

const MAX_SOURCE: usize = 1_048_576;
const MAX_TOKENS: usize = 65_536;
const MAX_STATEMENTS: usize = 16_384;
const MAX_DEPTH: usize = 64;
const MAX_VARIABLES: usize = 1_024;
const MAX_TEXT: usize = 1_048_576;
const MAX_STEPS: u64 = 10_000_000;
const MAX_SLICE: usize = 10_000;
const MAX_RETAINED_BYTES: usize = 16_777_216;
const MAX_VALUE_ELEMENTS: usize = 200_000;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Empty,
    Number(f64),
    Text(String),
    Bool(bool),
    Object(u64),
    Array(Vec<Vec<Value>>),
}

#[derive(Clone, Debug)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl Error {
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self { line, message: message.into() }
    }

    fn at(mut self, line: usize) -> Self {
        if self.line == 0 {
            self.line = line;
        }
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for Error {}

pub trait Host {
    fn get(&mut self, receiver: Option<&Value>, name: &str, args: &[Value]) -> Result<Value, Error>;

    fn set(&mut self, receiver: Option<&Value>, name: &str, args: &[Value], value: Value) -> Result<(), Error>;
}

#[derive(Clone, Debug)]
enum Token {
    Word(String),
    Number(f64),
    Text(String),
    Symbol(String),
}

impl Token {
    fn is(&self, expected: &str) -> bool {
        match self {
            Self::Word(s) | Self::Symbol(s) => s.eq_ignore_ascii_case(expected),
            _ => false,
        }
    }

    fn word(&self) -> Option<&str> {
        if let Self::Word(s) = self { Some(s) } else { None }
    }
}

#[derive(Clone, Debug)]
enum Expr {
    Literal(Value),
    Name(String),
    Member(Box<Expr>, String),
    WithMember(String),
    Apply(Box<Expr>, Vec<Expr>),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug)]
enum VariableType {
    Variant,
    Number,
    Single,
    Integer,
    Long,
    Byte,
    Text,
    Bool,
    Object,
}

impl VariableType {
    fn initial(self) -> Value {
        match self {
            Self::Variant | Self::Object => Value::Empty,
            Self::Text => Value::Text(String::new()),
            Self::Bool => Value::Bool(false),
            _ => Value::Number(0.0),
        }
    }

    fn convert(self, value: Value) -> Result<Value, Error> {
        match self {
            Self::Variant => Ok(value),
            Self::Object if matches!(value, Value::Object(_) | Value::Empty) => Ok(value),
            Self::Object => Err(Error::new(0, "An object variable requires Set and an object reference")),
            Self::Text => Ok(Value::Text(value_text(value)?)),
            Self::Bool => Ok(Value::Bool(value_bool(value)?)),
            Self::Number => checked_number(value_number(value)?),
            Self::Single => checked_number(f64::from(value_number(value)? as f32)),
            Self::Integer | Self::Long | Self::Byte => {
                let n = integer(value)?;
                let valid = match self {
                    Self::Integer => (i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&n),
                    Self::Byte => (0..=255).contains(&n),
                    _ => true,
                };
                if !valid {
                    return Err(Error::new(0, "Integer overflow"));
                }
                Ok(Value::Number(f64::from(n)))
            }
        }
    }
}

#[derive(Clone, Debug)]
enum Instruction {
    Declare(Vec<(String, VariableType)>),
    Assign(Expr, Expr, bool),
    Evaluate(Expr),
    Jump(usize),
    JumpIfFalse(Expr, usize),
    ForStart { name: String, from: Expr, to: Expr, step: Expr, exit: usize },
    ForNext { name: String, body: usize },
    EachStart { name: String, collection: Expr, exit: usize },
    EachNext { name: String, body: usize },
    LeaveLoop { target: usize, with_depth: usize, for_start: Option<usize>, for_depth: usize },
    WithStart(Expr),
    WithEnd,
    Return,
}

#[derive(Clone, Debug)]
struct Located {
    line: usize,
    instruction: Instruction,
}

#[derive(Clone, Debug)]
struct Procedure {
    name: String,
    code: Vec<Located>,
    variables: BTreeMap<String, VariableType>,
}

#[derive(Clone, Debug)]
pub struct Program {
    procedures: Vec<Procedure>,
    explicit: bool,
}

#[derive(Debug)]
struct ForState {
    start: usize,
    name: String,
    end: f64,
    step: f64,
    collection: Option<Value>,
    index: u64,
}

#[derive(Debug)]
struct Frame {
    procedure: usize,
    pc: usize,
    vars: BTreeMap<String, Value>,
    loops: Vec<ForState>,
    with: Vec<Value>,
    bytes: usize,
}

#[derive(Debug)]
pub struct Execution {
    program: Program,
    frames: Vec<Frame>,
    line: usize,
    steps: u64,
    failure: Option<Error>,
    retained_bytes: usize,
}

#[derive(Debug)]
enum Block {
    If { pending: Option<usize>, ends: Vec<usize>, had_else: bool },
    For { start: usize, name: String, exits: Vec<usize>, with_depth: usize, each: bool },
    Do { start: usize, condition: Option<usize>, exits: Vec<usize>, with_depth: usize, for_depth: usize },
    While { start: usize, condition: usize },
    With,
}

fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn is_at(tokens: &[Token], at: usize, text: &str) -> bool {
    tokens.get(at).is_some_and(|token| token.is(text))
}

fn word_at(tokens: &[Token], at: usize, line: usize) -> Result<String, Error> {
    tokens.get(at).and_then(Token::word).map(str::to_string).ok_or_else(|| Error::new(line, "Expected an identifier"))
}

fn slice(tokens: &[Token], start: usize, end: usize) -> &[Token] {
    tokens.get(start..end).unwrap_or_default()
}

/// Split logical lines without treating colons or apostrophes inside strings as
/// statement separators. Every generated instruction retains its source line.
fn statements(source: &str) -> Result<Vec<(usize, String)>, Error> {
    if source.len() > MAX_SOURCE {
        return Err(Error::new(1, "VBA source exceeds the 1 MiB limit"));
    }
    let mut result = Vec::new();
    let mut continued = String::new();
    let mut first_line = 1;
    for (index, original) in source.lines().enumerate() {
        let line = index.saturating_add(1);
        if continued.is_empty() {
            first_line = line;
        }
        let mut quoted = false;
        let mut chars = original.chars().peekable();
        let mut text = String::new();
        if original.trim_start().get(..3).is_some_and(|s| s.eq_ignore_ascii_case("rem"))
            && original.trim_start().chars().nth(3).is_none_or(char::is_whitespace)
        {
            continue;
        }
        while let Some(c) = chars.next() {
            if c == '"' {
                text.push(c);
                if quoted && chars.peek() == Some(&'"') {
                    text.push('"');
                    chars.next();
                } else {
                    quoted = !quoted;
                }
            } else if c == '\'' && !quoted {
                break;
            } else {
                if !quoted && c.is_whitespace() && text.rsplit(':').next().is_some_and(|part| part.trim().eq_ignore_ascii_case("rem")) {
                    break;
                }
                text.push(c);
            }
        }
        if quoted {
            return Err(Error::new(line, "Unterminated string literal"));
        }
        let trimmed = text.trim_end();
        if trimmed.ends_with(" _") || trimmed.ends_with("\t_") {
            let part = trimmed.get(..trimmed.len().saturating_sub(1)).unwrap_or_default();
            continued.push_str(part);
            continued.push(' ');
            continue;
        }
        continued.push_str(trimmed);
        let mut part = String::new();
        let mut inside = false;
        let mut chars = continued.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                part.push(c);
                if inside && chars.peek() == Some(&'"') {
                    part.push('"');
                    chars.next();
                } else {
                    inside = !inside;
                }
            } else if c == ':' && !inside {
                let tokens = tokenize(part.trim(), first_line)?;
                if is_at(&tokens, 0, "if") && top_level(&tokens, "then").is_some_and(|at| at.saturating_add(1) < tokens.len()) {
                    return Err(Error::new(first_line, "Multiple statements in a single-line If are not supported yet; use a block If"));
                }
                push_statement(&mut result, first_line, &part)?;
                part.clear();
            } else {
                part.push(c);
            }
        }
        push_statement(&mut result, first_line, &part)?;
        continued.clear();
    }
    if !continued.is_empty() {
        return Err(Error::new(first_line, "Unfinished line continuation"));
    }
    Ok(result)
}

fn push_statement(result: &mut Vec<(usize, String)>, line: usize, text: &str) -> Result<(), Error> {
    let text = text.trim();
    if !text.is_empty() && !text.eq_ignore_ascii_case("rem") && !text.get(..4).is_some_and(|s| s.eq_ignore_ascii_case("rem ")) {
        if result.len() >= MAX_STATEMENTS {
            return Err(Error::new(line, "Too many VBA statements"));
        }
        result.push((line, text.to_string()));
    }
    Ok(())
}

fn tokenize(text: &str, line: usize) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = 0;
    let mut result = Vec::new();
    while let Some(&c) = chars.get(pos) {
        if c.is_whitespace() {
            pos += 1;
            continue;
        }
        if result.len() >= MAX_TOKENS {
            return Err(Error::new(line, "Too many VBA tokens"));
        }
        if c == '"' {
            pos += 1;
            let mut value = String::new();
            let mut closed = false;
            while let Some(&next) = chars.get(pos) {
                pos += 1;
                if next == '"' {
                    if chars.get(pos) == Some(&'"') {
                        value.push('"');
                        pos += 1;
                    } else {
                        closed = true;
                        break;
                    }
                } else {
                    value.push(next);
                }
            }
            if !closed {
                return Err(Error::new(line, "Unterminated string literal"));
            }
            result.push(Token::Text(value));
        } else if c.is_alphabetic() || c == '_' {
            let start = pos;
            pos += 1;
            while chars.get(pos).is_some_and(|c| c.is_alphanumeric() || *c == '_') {
                pos += 1;
            }
            if pos.saturating_sub(start) > 255 {
                return Err(Error::new(line, "An identifier exceeds VBA's 255-character limit"));
            }
            result.push(Token::Word(chars.get(start..pos).unwrap_or_default().iter().collect()));
        } else if c.is_ascii_digit() || (c == '.' && chars.get(pos.saturating_add(1)).is_some_and(char::is_ascii_digit)) {
            let start = pos;
            let mut dot = false;
            while let Some(&n) = chars.get(pos) {
                if n.is_ascii_digit() {
                    pos += 1;
                } else if n == '.' && !dot {
                    dot = true;
                    pos += 1;
                } else {
                    break;
                }
            }
            if chars.get(pos).is_some_and(|c| *c == 'e' || *c == 'E') {
                pos += 1;
                if chars.get(pos).is_some_and(|c| *c == '+' || *c == '-') {
                    pos += 1;
                }
                while chars.get(pos).is_some_and(char::is_ascii_digit) {
                    pos += 1;
                }
            }
            let number: String = chars.get(start..pos).unwrap_or_default().iter().collect();
            let value = number.parse::<f64>().map_err(|_| Error::new(line, "Invalid numeric literal"))?;
            if !value.is_finite() {
                return Err(Error::new(line, "Numeric literal overflow"));
            }
            result.push(Token::Number(value));
        } else if "().,+-*/\\^&=<>".contains(c) {
            pos += 1;
            let mut symbol = c.to_string();
            if (c == '<' && chars.get(pos).is_some_and(|n| *n == '=' || *n == '>')) || (c == '>' && chars.get(pos) == Some(&'=')) {
                if let Some(next) = chars.get(pos) {
                    symbol.push(*next);
                }
                pos += 1;
            }
            result.push(Token::Symbol(symbol));
        } else {
            return Err(Error::new(line, format!("Unsupported character: {c}")));
        }
    }
    Ok(result)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    line: usize,
}

impl Parser<'_> {
    fn expression(&mut self, minimum: u8, depth: usize) -> Result<Expr, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::new(self.line, "Expression nesting exceeds the limit"));
        }
        let token = self.tokens.get(self.pos).cloned().ok_or_else(|| Error::new(self.line, "Expected an expression"))?;
        self.pos += 1;
        let mut left = match token {
            Token::Number(n) => Expr::Literal(Value::Number(n)),
            Token::Text(s) => Expr::Literal(Value::Text(s)),
            Token::Word(s) if s.eq_ignore_ascii_case("true") => Expr::Literal(Value::Bool(true)),
            Token::Word(s) if s.eq_ignore_ascii_case("false") => Expr::Literal(Value::Bool(false)),
            Token::Word(s) if s.eq_ignore_ascii_case("empty") || s.eq_ignore_ascii_case("nothing") => Expr::Literal(Value::Empty),
            Token::Word(s) if s.eq_ignore_ascii_case("not") => Expr::Unary(key(&s), Box::new(self.expression(4, depth.saturating_add(1))?)),
            Token::Word(s) => Expr::Name(s),
            Token::Symbol(s) if s == "+" || s == "-" => Expr::Unary(s, Box::new(self.expression(11, depth.saturating_add(1))?)),
            Token::Symbol(s) if s == "(" => {
                let nested = self.expression(0, depth.saturating_add(1))?;
                self.consume(")")?;
                nested
            }
            Token::Symbol(s) if s == "." => {
                let name = word_at(self.tokens, self.pos, self.line)?;
                self.pos += 1;
                Expr::WithMember(name)
            }
            _ => return Err(Error::new(self.line, "Expected an expression")),
        };
        let mut chain = depth;
        loop {
            if is_at(self.tokens, self.pos, ".") {
                chain = chain.saturating_add(1);
                if chain > MAX_DEPTH {
                    return Err(Error::new(self.line, "Member nesting exceeds the limit"));
                }
                self.pos += 1;
                let name = word_at(self.tokens, self.pos, self.line)?;
                self.pos += 1;
                left = Expr::Member(Box::new(left), name);
                continue;
            }
            if is_at(self.tokens, self.pos, "(") {
                chain = chain.saturating_add(1);
                if chain > MAX_DEPTH {
                    return Err(Error::new(self.line, "Call nesting exceeds the limit"));
                }
                self.pos += 1;
                let mut args = Vec::new();
                if !is_at(self.tokens, self.pos, ")") {
                    loop {
                        if args.len() >= 255 {
                            return Err(Error::new(self.line, "Too many call arguments"));
                        }
                        args.push(self.expression(0, chain.saturating_add(1))?);
                        if !is_at(self.tokens, self.pos, ",") {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                self.consume(")")?;
                left = Expr::Apply(Box::new(left), args);
                continue;
            }
            let Some(token) = self.tokens.get(self.pos) else { break };
            let op = match token {
                Token::Word(s) | Token::Symbol(s) => key(s),
                _ => break,
            };
            let precedence = match op.as_str() {
                "or" | "xor" => 1,
                "and" => 2,
                "=" | "<>" | "<" | ">" | "<=" | ">=" | "is" => 4,
                "&" => 5,
                "+" | "-" => 6,
                "mod" => 7,
                "\\" => 8,
                "*" | "/" => 9,
                "^" => 12,
                _ => break,
            };
            if precedence < minimum {
                break;
            }
            chain = chain.saturating_add(1);
            if chain > MAX_DEPTH {
                return Err(Error::new(self.line, "Expression nesting exceeds the limit"));
            }
            self.pos += 1;
            let right = self.expression(precedence.saturating_add(1), chain)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        check_expression_depth(&left, 0, self.line)?;
        Ok(left)
    }

    fn consume(&mut self, expected: &str) -> Result<(), Error> {
        if !is_at(self.tokens, self.pos, expected) {
            return Err(Error::new(self.line, format!("Expected '{expected}'")));
        }
        self.pos += 1;
        Ok(())
    }
}

fn check_expression_depth(expr: &Expr, depth: usize, line: usize) -> Result<(), Error> {
    if depth > MAX_DEPTH {
        return Err(Error::new(line, "Expression nesting exceeds the limit"));
    }
    let next = depth.saturating_add(1);
    match expr {
        Expr::Member(receiver, _) | Expr::Unary(_, receiver) => check_expression_depth(receiver, next, line),
        Expr::Apply(callee, args) => {
            check_expression_depth(callee, next, line)?;
            for arg in args {
                check_expression_depth(arg, next, line)?;
            }
            Ok(())
        }
        Expr::Binary(_, left, right) => {
            check_expression_depth(left, next, line)?;
            check_expression_depth(right, next, line)
        }
        _ => Ok(()),
    }
}

fn expression(tokens: &[Token], line: usize) -> Result<Expr, Error> {
    let mut parser = Parser { tokens, pos: 0, line };
    let expr = parser.expression(0, 0)?;
    if parser.pos != tokens.len() {
        return Err(Error::new(line, "Unexpected text after expression"));
    }
    Ok(expr)
}

fn top_level(tokens: &[Token], name: &str) -> Option<usize> {
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate() {
        if token.is("(") {
            depth = depth.saturating_add(1);
        } else if token.is(")") {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && token.is(name) {
            return Some(index);
        }
    }
    None
}

fn emit(code: &mut Vec<Located>, line: usize, instruction: Instruction) -> usize {
    let at = code.len();
    code.push(Located { line, instruction });
    at
}

fn patch(code: &mut [Located], at: usize, target: usize) -> Result<(), Error> {
    let located = code.get_mut(at).ok_or_else(|| Error::new(0, "Invalid compiler jump"))?;
    match &mut located.instruction {
        Instruction::Jump(to) | Instruction::JumpIfFalse(_, to) => *to = target,
        Instruction::ForStart { exit, .. } | Instruction::EachStart { exit, .. } => *exit = target,
        Instruction::LeaveLoop { target: to, .. } => *to = target,
        _ => return Err(Error::new(located.line, "Invalid compiler jump target")),
    }
    Ok(())
}

fn default_type(tokens: &[Token], line: usize) -> Result<VariableType, Error> {
    if tokens.is_empty() {
        return Ok(VariableType::Variant);
    }
    if tokens.len() != 2 || !is_at(tokens, 0, "as") {
        return Err(Error::new(line, "Only scalar Dim declarations are currently supported"));
    }
    let name = key(&word_at(tokens, 1, line)?);
    match name.as_str() {
        "variant" => Ok(VariableType::Variant),
        "object" | "range" | "worksheet" | "workbook" => Ok(VariableType::Object),
        "string" => Ok(VariableType::Text),
        "boolean" => Ok(VariableType::Bool),
        "double" => Ok(VariableType::Number),
        "single" => Ok(VariableType::Single),
        "integer" => Ok(VariableType::Integer),
        "long" => Ok(VariableType::Long),
        "byte" => Ok(VariableType::Byte),
        _ => Err(Error::new(line, format!("Unsupported declaration type: {name}"))),
    }
}

fn simple(tokens: &[Token], line: usize, code: &mut Vec<Located>, blocks: &mut [Block]) -> Result<(), Error> {
    if tokens.is_empty() {
        return Err(Error::new(line, "Expected a statement"));
    }
    if is_at(tokens, 0, "dim") {
        let mut variables = Vec::new();
        let mut start = 1;
        for end in 1..=tokens.len() {
            if end == tokens.len() || is_at(tokens, end, ",") {
                let segment = slice(tokens, start, end);
                let name = word_at(segment, 0, line)?;
                variables.push((key(&name), default_type(slice(segment, 1, segment.len()), line)?));
                start = end.saturating_add(1);
            }
        }
        emit(code, line, Instruction::Declare(variables));
        return Ok(());
    }
    let is_set = is_at(tokens, 0, "set");
    let assignment_prefix = is_set || is_at(tokens, 0, "let");
    let body = if assignment_prefix { slice(tokens, 1, tokens.len()) } else { tokens };
    if let Some(equal) = top_level(body, "=") {
        match expression(slice(body, 0, equal), line) {
            Ok(target) => {
                if !matches!(target, Expr::Name(_) | Expr::Member(_, _) | Expr::WithMember(_) | Expr::Apply(_, _)) {
                    return Err(Error::new(line, "Invalid assignment target"));
                }
                let value = expression(slice(body, equal.saturating_add(1), body.len()), line)?;
                emit(code, line, Instruction::Assign(target, value, is_set));
                return Ok(());
            }
            Err(error) if assignment_prefix => return Err(error),
            // e.g. Debug.Print x = 1 is a comparison passed to a call.
            Err(_) => {}
        }
    }
    if assignment_prefix {
        return Err(Error::new(line, "Expected '=' in assignment"));
    }
    if is_at(tokens, 0, "exit") && is_at(tokens, 1, "sub") && tokens.len() == 2 {
        emit(code, line, Instruction::Return);
        return Ok(());
    }
    if is_at(tokens, 0, "exit") && tokens.len() == 2 {
        return exit_loop(tokens, line, code, blocks);
    }
    let body = if is_at(tokens, 0, "call") { slice(tokens, 1, tokens.len()) } else { tokens };
    let forbidden = [
        "end",
        "stop",
        "on",
        "resume",
        "goto",
        "gosub",
        "select",
        "case",
        "redim",
        "erase",
        "const",
        "static",
        "public",
        "private",
        "function",
        "raiseevent",
        "open",
        "close",
        "print",
        "input",
        "write",
        "kill",
        "shell",
        "chdir",
        "mkdir",
        "rmdir",
    ];
    if body.first().and_then(Token::word).is_some_and(|w| forbidden.iter().any(|f| w.eq_ignore_ascii_case(f))) {
        return Err(Error::new(line, "This VBA statement is not supported yet"));
    }
    // A VBA statement call can omit parentheses: Debug.Print "a", 2.
    let mut parser = Parser { tokens: body, pos: 0, line };
    let mut call = parser.expression(13, 0)?;
    if parser.pos < body.len() {
        let mut args = Vec::new();
        loop {
            if args.len() >= 255 {
                return Err(Error::new(line, "Too many call arguments"));
            }
            args.push(parser.expression(0, 0)?);
            if parser.pos == body.len() {
                break;
            }
            parser.consume(",")?;
        }
        if matches!(call, Expr::Apply(_, _)) {
            return Err(Error::new(line, "Unexpected arguments after a parenthesized call"));
        }
        call = Expr::Apply(Box::new(call), args);
    }
    if !matches!(call, Expr::Name(_) | Expr::Member(_, _) | Expr::WithMember(_) | Expr::Apply(_, _)) {
        return Err(Error::new(line, "Expected a procedure call"));
    }
    emit(code, line, Instruction::Evaluate(call));
    Ok(())
}

impl Program {
    pub fn compile(source: &str) -> Result<Self, Error> {
        let mut program = Self { procedures: Vec::new(), explicit: false };
        let mut current: Option<Procedure> = None;
        let mut blocks: Vec<Block> = Vec::new();
        let mut token_count = 0_usize;
        for (line, text) in statements(source)? {
            let tokens = tokenize(&text, line)?;
            token_count = token_count.saturating_add(tokens.len());
            if token_count > MAX_TOKENS {
                return Err(Error::new(line, "Too many VBA tokens"));
            }
            if tokens.is_empty() {
                continue;
            }
            if is_at(&tokens, 0, "option") {
                if current.is_some() || tokens.len() != 2 || !is_at(&tokens, 1, "explicit") {
                    return Err(Error::new(line, "Only module-level Option Explicit is supported"));
                }
                program.explicit = true;
                continue;
            }
            if is_at(&tokens, 0, "attribute") {
                if current.is_some() || tokens.len() != 4 || !is_at(&tokens, 2, "=") {
                    return Err(Error::new(line, "Unsupported VBA module Attribute"));
                }
                let name = key(&word_at(&tokens, 1, line)?);
                let valid = match name.as_str() {
                    "vb_name" | "vb_description" => matches!(tokens.get(3), Some(Token::Text(_))),
                    "vb_globalnamespace" | "vb_creatable" | "vb_predeclaredid" | "vb_exposed" => {
                        is_at(&tokens, 3, "true") || is_at(&tokens, 3, "false")
                    }
                    _ => false,
                };
                if !valid {
                    return Err(Error::new(line, "Unsupported VBA module Attribute"));
                }
                // These exported module metadata fields do not execute.
                continue;
            }
            let prefix = usize::from(is_at(&tokens, 0, "public") || is_at(&tokens, 0, "private"));
            if is_at(&tokens, prefix, "sub") {
                if current.is_some() {
                    return Err(Error::new(line, "Nested Sub declarations are invalid"));
                }
                let name = word_at(&tokens, prefix.saturating_add(1), line)?;
                let rest = slice(&tokens, prefix.saturating_add(2), tokens.len());
                if !(rest.is_empty() || (rest.len() == 2 && is_at(rest, 0, "(") && is_at(rest, 1, ")"))) {
                    return Err(Error::new(line, "Sub parameters are not supported yet"));
                }
                if program.procedures.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
                    return Err(Error::new(line, "Duplicate procedure name"));
                }
                current = Some(Procedure { name, code: Vec::new(), variables: BTreeMap::new() });
                continue;
            }
            if is_at(&tokens, 0, "end") && is_at(&tokens, 1, "sub") {
                if tokens.len() != 2 {
                    return Err(Error::new(line, "Unexpected text after End Sub"));
                }
                if !blocks.is_empty() {
                    return Err(Error::new(line, "An If, loop, or With block is not closed"));
                }
                let mut procedure = current.take().ok_or_else(|| Error::new(line, "End Sub without Sub"))?;
                emit(&mut procedure.code, line, Instruction::Return);
                for located in &procedure.code {
                    if let Instruction::Declare(declarations) = &located.instruction {
                        for (name, kind) in declarations {
                            if procedure.variables.len() >= MAX_VARIABLES {
                                return Err(Error::new(located.line, "Too many variables"));
                            }
                            if procedure.variables.insert(name.clone(), *kind).is_some() {
                                return Err(Error::new(located.line, "Duplicate variable declaration"));
                            }
                        }
                    }
                }
                program.procedures.push(procedure);
                continue;
            }
            let procedure = current
                .as_mut()
                .ok_or_else(|| Error::new(line, "Expected a Sub declaration (module variables and Function are not supported yet)"))?;
            let code = &mut procedure.code;
            compile_statement(&tokens, line, code, &mut blocks)?;
            if blocks.len() > MAX_DEPTH {
                return Err(Error::new(line, "Block nesting exceeds the limit"));
            }
            if code.len() > MAX_STATEMENTS {
                return Err(Error::new(line, "Too many compiled instructions"));
            }
        }
        if current.is_some() {
            return Err(Error::new(source.lines().count().max(1), "Missing End Sub"));
        }
        if program.procedures.is_empty() {
            return Err(Error::new(1, "No Sub procedure was found"));
        }
        Ok(program)
    }

    pub fn procedures(&self) -> Vec<String> {
        self.procedures.iter().map(|p| p.name.clone()).collect()
    }

    pub fn start(&self, name: &str) -> Result<Execution, Error> {
        let index = self
            .procedures
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| Error::new(0, format!("Procedure not found: {name}")))?;
        let frame = self.create_frame(index)?;
        let retained_bytes = frame.bytes;
        Ok(Execution { program: self.clone(), frames: vec![frame], line: 0, steps: 0, failure: None, retained_bytes })
    }

    fn create_frame(&self, index: usize) -> Result<Frame, Error> {
        let procedure = self.procedures.get(index).ok_or_else(|| Error::new(0, "Procedure does not exist"))?;
        let vars: BTreeMap<String, Value> = procedure.variables.iter().map(|(name, kind)| (name.clone(), kind.initial())).collect();
        let mut bytes = 0_usize;
        for (name, value) in &vars {
            bytes = bytes.saturating_add(name.len()).saturating_add(value_size(value)?);
        }
        Ok(Frame { procedure: index, pc: 0, vars, loops: Vec::new(), with: Vec::new(), bytes })
    }
}

fn compile_statement(tokens: &[Token], line: usize, code: &mut Vec<Located>, blocks: &mut Vec<Block>) -> Result<(), Error> {
    let with_depth = blocks.iter().filter(|b| matches!(b, Block::With)).count();
    if is_at(tokens, 0, "if") {
        let then = top_level(tokens, "then").ok_or_else(|| Error::new(line, "Expected Then"))?;
        let condition = expression(slice(tokens, 1, then), line)?;
        let jump = emit(code, line, Instruction::JumpIfFalse(condition, 0));
        if then.saturating_add(1) == tokens.len() {
            blocks.push(Block::If { pending: Some(jump), ends: Vec::new(), had_else: false });
        } else {
            let tail = slice(tokens, then.saturating_add(1), tokens.len());
            if let Some(otherwise) = top_level(tail, "else") {
                simple(slice(tail, 0, otherwise), line, code, blocks)?;
                let end = emit(code, line, Instruction::Jump(0));
                let target = code.len();
                patch(code, jump, target)?;
                simple(slice(tail, otherwise.saturating_add(1), tail.len()), line, code, blocks)?;
                let target = code.len();
                patch(code, end, target)?;
            } else {
                simple(tail, line, code, blocks)?;
                let target = code.len();
                patch(code, jump, target)?;
            }
        }
    } else if is_at(tokens, 0, "elseif") || is_at(tokens, 0, "else") {
        let Some(Block::If { pending, ends, had_else }) = blocks.last_mut() else {
            return Err(Error::new(line, "Else/ElseIf without a matching If"));
        };
        if *had_else {
            return Err(Error::new(line, "ElseIf/Else cannot follow Else"));
        }
        let end = emit(code, line, Instruction::Jump(0));
        ends.push(end);
        if let Some(old) = pending.take() {
            let target = code.len();
            patch(code, old, target)?;
        }
        if is_at(tokens, 0, "elseif") {
            let then = top_level(tokens, "then").ok_or_else(|| Error::new(line, "Expected Then"))?;
            if then.saturating_add(1) != tokens.len() {
                return Err(Error::new(line, "Use a block ElseIf with Then at the end of its line"));
            }
            let condition = expression(slice(tokens, 1, then), line)?;
            *pending = Some(emit(code, line, Instruction::JumpIfFalse(condition, 0)));
        } else {
            if tokens.len() != 1 {
                return Err(Error::new(line, "Unexpected text after Else"));
            }
            *had_else = true;
        }
    } else if is_at(tokens, 0, "end") && is_at(tokens, 1, "if") && tokens.len() == 2 {
        let Some(Block::If { pending, ends, .. }) = blocks.pop() else {
            return Err(Error::new(line, "End If without a matching If"));
        };
        let target = code.len();
        if let Some(at) = pending {
            patch(code, at, target)?;
        }
        for at in ends {
            patch(code, at, target)?;
        }
    } else if is_at(tokens, 0, "for") {
        if is_at(tokens, 1, "each") {
            let name = key(&word_at(tokens, 2, line)?);
            if blocks.iter().any(|block| matches!(block, Block::For { name: active, .. } if *active == name)) {
                return Err(Error::new(line, "For control variable is already in use"));
            }
            if !is_at(tokens, 3, "in") {
                return Err(Error::new(line, "Expected In after For Each variable"));
            }
            let collection = expression(slice(tokens, 4, tokens.len()), line)?;
            let start = emit(code, line, Instruction::EachStart { name: name.clone(), collection, exit: 0 });
            blocks.push(Block::For { start, name, exits: Vec::new(), with_depth, each: true });
            return Ok(());
        }
        let name = key(&word_at(tokens, 1, line)?);
        if blocks.iter().any(|block| matches!(block, Block::For { name: active, .. } if *active == name)) {
            return Err(Error::new(line, "For control variable is already in use"));
        }
        if !is_at(tokens, 2, "=") {
            return Err(Error::new(line, "Expected '=' after For variable"));
        }
        let to = top_level(tokens, "to").ok_or_else(|| Error::new(line, "Expected To in For statement"))?;
        let step_at = top_level(tokens, "step");
        let end_at = step_at.unwrap_or(tokens.len());
        let from = expression(slice(tokens, 3, to), line)?;
        let to = expression(slice(tokens, to.saturating_add(1), end_at), line)?;
        let step = match step_at {
            Some(at) => expression(slice(tokens, at.saturating_add(1), tokens.len()), line)?,
            None => Expr::Literal(Value::Number(1.0)),
        };
        let start = emit(code, line, Instruction::ForStart { name: name.clone(), from, to, step, exit: 0 });
        blocks.push(Block::For { start, name, exits: Vec::new(), with_depth, each: false });
    } else if is_at(tokens, 0, "next") {
        let Some(Block::For { start, name, exits, each, .. }) = blocks.pop() else {
            return Err(Error::new(line, "Next without a matching For"));
        };
        if tokens.len() > 2 || (tokens.len() == 2 && !word_at(tokens, 1, line)?.eq_ignore_ascii_case(&name)) {
            return Err(Error::new(line, "Next variable does not match For"));
        }
        emit(
            code,
            line,
            if each {
                Instruction::EachNext { name, body: start.saturating_add(1) }
            } else {
                Instruction::ForNext { name, body: start.saturating_add(1) }
            },
        );
        let target = code.len();
        patch(code, start, target)?;
        for at in exits {
            patch(code, at, target)?;
        }
    } else if is_at(tokens, 0, "do") {
        let start = code.len();
        let condition = if tokens.len() == 1 {
            None
        } else if is_at(tokens, 1, "while") || is_at(tokens, 1, "until") {
            let mut condition = expression(slice(tokens, 2, tokens.len()), line)?;
            if is_at(tokens, 1, "until") {
                condition = Expr::Unary("condition_not".to_string(), Box::new(condition));
            }
            Some(emit(code, line, Instruction::JumpIfFalse(condition, 0)))
        } else {
            return Err(Error::new(line, "Expected While or Until after Do"));
        };
        let for_depth = blocks.iter().filter(|block| matches!(block, Block::For { .. })).count();
        blocks.push(Block::Do { start, condition, exits: Vec::new(), with_depth, for_depth });
    } else if is_at(tokens, 0, "loop") {
        let Some(Block::Do { start, condition, exits, .. }) = blocks.pop() else {
            return Err(Error::new(line, "Loop without a matching Do"));
        };
        if tokens.len() == 1 {
            emit(code, line, Instruction::Jump(start));
        } else if is_at(tokens, 1, "while") || is_at(tokens, 1, "until") {
            if condition.is_some() {
                return Err(Error::new(line, "A Do loop cannot have both a leading and trailing condition"));
            }
            let mut value = expression(slice(tokens, 2, tokens.len()), line)?;
            if is_at(tokens, 1, "while") {
                value = Expr::Unary("condition_not".to_string(), Box::new(value));
            }
            emit(code, line, Instruction::JumpIfFalse(value, start));
        } else {
            return Err(Error::new(line, "Expected While or Until after Loop"));
        }
        let target = code.len();
        if let Some(at) = condition {
            patch(code, at, target)?;
        }
        for at in exits {
            patch(code, at, target)?;
        }
    } else if is_at(tokens, 0, "while") {
        let start = code.len();
        let value = expression(slice(tokens, 1, tokens.len()), line)?;
        let condition = emit(code, line, Instruction::JumpIfFalse(value, 0));
        blocks.push(Block::While { start, condition });
    } else if is_at(tokens, 0, "wend") && tokens.len() == 1 {
        let Some(Block::While { start, condition }) = blocks.pop() else {
            return Err(Error::new(line, "Wend without a matching While"));
        };
        emit(code, line, Instruction::Jump(start));
        let target = code.len();
        patch(code, condition, target)?;
    } else if is_at(tokens, 0, "exit") && tokens.len() == 2 && (is_at(tokens, 1, "for") || is_at(tokens, 1, "do")) {
        exit_loop(tokens, line, code, blocks)?;
    } else if is_at(tokens, 0, "with") {
        emit(code, line, Instruction::WithStart(expression(slice(tokens, 1, tokens.len()), line)?));
        blocks.push(Block::With);
    } else if is_at(tokens, 0, "end") && is_at(tokens, 1, "with") && tokens.len() == 2 {
        if !matches!(blocks.pop(), Some(Block::With)) {
            return Err(Error::new(line, "End With without a matching With"));
        }
        emit(code, line, Instruction::WithEnd);
    } else {
        simple(tokens, line, code, blocks)?;
    }
    Ok(())
}

fn exit_loop(tokens: &[Token], line: usize, code: &mut Vec<Located>, blocks: &mut [Block]) -> Result<(), Error> {
    let is_for = is_at(tokens, 1, "for");
    if !is_for && !is_at(tokens, 1, "do") {
        return Err(Error::new(line, "Expected Sub, For, or Do after Exit"));
    }
    let target = blocks
        .iter_mut()
        .rev()
        .find(|block| (is_for && matches!(block, Block::For { .. })) || (!is_for && matches!(block, Block::Do { .. })))
        .ok_or_else(|| Error::new(line, "Exit without a matching loop"))?;
    match target {
        Block::For { start, exits, with_depth, .. } => {
            let at = emit(code, line, Instruction::LeaveLoop { target: 0, with_depth: *with_depth, for_start: Some(*start), for_depth: 0 });
            exits.push(at);
        }
        Block::Do { exits, with_depth, for_depth, .. } => {
            let at = emit(code, line, Instruction::LeaveLoop { target: 0, with_depth: *with_depth, for_start: None, for_depth: *for_depth });
            exits.push(at);
        }
        _ => return Err(Error::new(line, "Invalid Exit loop target")),
    }
    Ok(())
}

fn value_number(value: Value) -> Result<f64, Error> {
    let result = match value {
        Value::Number(n) => n,
        Value::Bool(true) => -1.0,
        Value::Bool(false) | Value::Empty => 0.0,
        Value::Text(text) => text.trim().parse().map_err(|_| Error::new(0, "Type mismatch: expected a number"))?,
        _ => return Err(Error::new(0, "Type mismatch: expected a scalar number")),
    };
    if result.is_finite() { Ok(result) } else { Err(Error::new(0, "Numeric overflow")) }
}

fn value_text(value: Value) -> Result<String, Error> {
    match value {
        Value::Empty => Ok(String::new()),
        Value::Text(s) => Ok(s),
        Value::Number(n) if n.is_finite() => Ok(n.to_string()),
        Value::Bool(b) => Ok(if b { "True" } else { "False" }.to_string()),
        _ => Err(Error::new(0, "Type mismatch: expected a scalar value")),
    }
}

fn value_bool(value: Value) -> Result<bool, Error> {
    match value {
        Value::Bool(b) => Ok(b),
        Value::Text(s) if s.eq_ignore_ascii_case("true") => Ok(true),
        Value::Text(s) if s.eq_ignore_ascii_case("false") => Ok(false),
        other => Ok(value_number(other)? != 0.0),
    }
}

fn scalar(host: &mut impl Host, value: Value) -> Result<Value, Error> {
    if matches!(value, Value::Object(_)) {
        let resolved = host_get(host, Some(&value), "", &[])?;
        if matches!(resolved, Value::Object(_)) {
            return Err(Error::new(0, "Object default property did not return a value"));
        }
        Ok(resolved)
    } else {
        Ok(value)
    }
}

fn value_size(value: &Value) -> Result<usize, Error> {
    let mut pending = vec![(value, 0_usize)];
    let mut elements = 0_usize;
    let mut bytes = 0_usize;
    while let Some((value, depth)) = pending.pop() {
        if depth > MAX_DEPTH {
            return Err(Error::new(0, "Array nesting exceeds the limit"));
        }
        elements = elements.saturating_add(1);
        if elements.saturating_add(pending.len()) > MAX_VALUE_ELEMENTS {
            return Err(Error::new(0, "A value exceeds the 200,000-element limit"));
        }
        bytes = bytes.saturating_add(std::mem::size_of::<Value>());
        match value {
            Value::Text(text) => {
                if text.len() > MAX_TEXT {
                    return Err(Error::new(0, "Text exceeds the 1 MiB limit"));
                }
                bytes = bytes.saturating_add(text.len());
            }
            Value::Number(number) if !number.is_finite() => return Err(Error::new(0, "Numeric overflow")),
            Value::Array(rows) => {
                if rows.len() > MAX_VALUE_ELEMENTS {
                    return Err(Error::new(0, "Too many array rows"));
                }
                bytes = bytes.saturating_add(rows.len().saturating_mul(std::mem::size_of::<Vec<Value>>()));
                for row in rows {
                    if elements.saturating_add(pending.len()).saturating_add(row.len()) > MAX_VALUE_ELEMENTS {
                        return Err(Error::new(0, "A value exceeds the 200,000-element limit"));
                    }
                    pending.extend(row.iter().map(|value| (value, depth.saturating_add(1))));
                }
            }
            _ => {}
        }
        if bytes > MAX_RETAINED_BYTES {
            return Err(Error::new(0, "A value exceeds the 16 MiB memory limit"));
        }
    }
    Ok(bytes)
}

fn host_get(host: &mut impl Host, receiver: Option<&Value>, name: &str, args: &[Value]) -> Result<Value, Error> {
    let value = host.get(receiver, name, args)?;
    if let Err(error) = value_size(&value) {
        // Do not recursively drop a hostile, over-nested array returned by a
        // host. Its shape has just failed validation.
        discard_value(value);
        return Err(error);
    }
    Ok(value)
}

fn discard_value(value: Value) {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        if let Value::Array(rows) = value {
            for row in rows {
                pending.extend(row);
            }
        }
    }
}

fn checked_number(value: f64) -> Result<Value, Error> {
    if value.is_finite() { Ok(Value::Number(value)) } else { Err(Error::new(0, "Numeric overflow")) }
}

fn integer(value: Value) -> Result<i32, Error> {
    let n = value_number(value)?.round_ties_even();
    if n < f64::from(i32::MIN) || n > f64::from(i32::MAX) {
        return Err(Error::new(0, "Integer overflow"));
    }
    Ok(n as i32)
}

fn binary(host: &mut impl Host, op: &str, left: Value, right: Value) -> Result<Value, Error> {
    if op == "is" {
        return match (&left, &right) {
            (Value::Object(a), Value::Object(b)) => Ok(Value::Bool(a == b)),
            (Value::Empty, Value::Empty) => Ok(Value::Bool(true)),
            (Value::Object(_), Value::Empty) | (Value::Empty, Value::Object(_)) => Ok(Value::Bool(false)),
            _ => Err(Error::new(0, "Is requires object references")),
        };
    }
    let left = scalar(host, left)?;
    let right = scalar(host, right)?;
    if op == "&" || (op == "+" && matches!((&left, &right), (Value::Text(_), Value::Text(_)))) {
        let mut text = value_text(left)?;
        let other = value_text(right)?;
        if text.len().saturating_add(other.len()) > MAX_TEXT {
            return Err(Error::new(0, "Text exceeds the 1 MiB limit"));
        }
        text.push_str(&other);
        return Ok(Value::Text(text));
    }
    if matches!(op, "=" | "<>" | "<" | ">" | "<=" | ">=") {
        let order = match (&left, &right) {
            (Value::Text(_), Value::Text(_) | Value::Empty) | (Value::Empty, Value::Text(_)) => value_text(left)?.partial_cmp(&value_text(right)?),
            _ => value_number(left)?.partial_cmp(&value_number(right)?),
        }
        .ok_or_else(|| Error::new(0, "Values cannot be compared"))?;
        return Ok(Value::Bool(match op {
            "=" => order.is_eq(),
            "<>" => !order.is_eq(),
            "<" => order.is_lt(),
            ">" => order.is_gt(),
            "<=" => !order.is_gt(),
            ">=" => !order.is_lt(),
            _ => false,
        }));
    }
    if matches!(op, "and" | "or" | "xor") {
        if let (Value::Bool(a), Value::Bool(b)) = (&left, &right) {
            return Ok(Value::Bool(match op {
                "and" => *a && *b,
                "or" => *a || *b,
                _ => *a ^ *b,
            }));
        }
        let a = integer(left)?;
        let b = integer(right)?;
        return Ok(Value::Number(f64::from(match op {
            "and" => a & b,
            "or" => a | b,
            _ => a ^ b,
        })));
    }
    if op == "\\" || op == "mod" {
        let a = integer(left)?;
        let b = integer(right)?;
        if b == 0 {
            return Err(Error::new(0, "Division by zero"));
        }
        let result = if op == "\\" { a.checked_div(b) } else { a.checked_rem(b) }.ok_or_else(|| Error::new(0, "Integer overflow"))?;
        return Ok(Value::Number(f64::from(result)));
    }
    let a = value_number(left)?;
    let b = value_number(right)?;
    match op {
        "+" => checked_number(a + b),
        "-" => checked_number(a - b),
        "*" => checked_number(a * b),
        "/" if b != 0.0 => checked_number(a / b),
        "/" => Err(Error::new(0, "Division by zero")),
        "^" => checked_number(a.powf(b)),
        _ => Err(Error::new(0, "Unsupported operator")),
    }
}

impl Execution {
    pub fn line(&self) -> usize {
        self.line
    }

    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// Execute at most `budget` instructions, with a hard per-call ceiling so a
    /// caller cannot accidentally monopolize the browser's event loop.
    pub fn step(&mut self, host: &mut impl Host, budget: usize) -> Result<bool, Error> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        for _ in 0..budget.min(MAX_SLICE) {
            if self.frames.is_empty() {
                return Ok(true);
            }
            if self.steps >= MAX_STEPS {
                let error = Error::new(self.line, "Execution exceeded 10,000,000 instructions");
                self.failure = Some(error.clone());
                return Err(error);
            }
            if let Err(error) = self.run_instruction(host) {
                let error = error.at(self.line);
                self.failure = Some(error.clone());
                return Err(error);
            }
            self.steps = self.steps.saturating_add(1);
        }
        Ok(self.frames.is_empty())
    }

    fn frame(&self) -> Result<&Frame, Error> {
        self.frames.last().ok_or_else(|| Error::new(self.line, "No active procedure"))
    }

    fn frame_mut(&mut self) -> Result<&mut Frame, Error> {
        self.frames.last_mut().ok_or_else(|| Error::new(self.line, "No active procedure"))
    }

    fn jump(&mut self, target: usize) -> Result<(), Error> {
        self.frame_mut()?.pc = target;
        Ok(())
    }

    fn assign_variable(&mut self, name: &str, value: Value) -> Result<(), Error> {
        let explicit = self.program.explicit;
        let variable_type = self.variable_type(name)?;
        let value = variable_type.convert(value)?;
        let old = self.frame()?.vars.get(&key(name));
        let old_bytes = match old {
            Some(value) => value_size(value)?,
            None => 0,
        };
        let new_bytes = value_size(&value)?.saturating_add(if old.is_none() { key(name).len() } else { 0 });
        let retained_bytes = self.retained_bytes.saturating_sub(old_bytes).saturating_add(new_bytes);
        if retained_bytes > MAX_RETAINED_BYTES {
            return Err(Error::new(0, "Variables exceed the 16 MiB memory limit"));
        }
        let frame = self.frame_mut()?;
        let name = key(name);
        if !frame.vars.contains_key(&name) {
            if explicit {
                return Err(Error::new(0, format!("Variable not defined: {name}")));
            }
            if frame.vars.len() >= MAX_VARIABLES {
                return Err(Error::new(0, "Too many variables"));
            }
        }
        frame.vars.insert(name, value);
        frame.bytes = frame.bytes.saturating_sub(old_bytes).saturating_add(new_bytes);
        self.retained_bytes = retained_bytes;
        Ok(())
    }

    fn variable_type(&self, name: &str) -> Result<VariableType, Error> {
        Ok(self.program.procedures.get(self.frame()?.procedure).and_then(|p| p.variables.get(&key(name))).copied().unwrap_or(VariableType::Variant))
    }

    fn pop_frame(&mut self) {
        if let Some(frame) = self.frames.pop() {
            self.retained_bytes = self.retained_bytes.saturating_sub(frame.bytes);
        }
    }

    fn run_instruction(&mut self, host: &mut impl Host) -> Result<(), Error> {
        let frame = self.frame()?;
        let pc = frame.pc;
        let located = self.program.procedures.get(frame.procedure).and_then(|procedure| procedure.code.get(pc)).cloned();
        let Some(located) = located else {
            self.pop_frame();
            return Ok(());
        };
        self.line = located.line;
        self.frame_mut()?.pc = pc.saturating_add(1);
        match located.instruction {
            // Dim has procedure scope; declarations are initialized when the
            // frame is created, even if their source is inside a skipped If.
            Instruction::Declare(_) => {}
            Instruction::Assign(target, value, is_set) => {
                let mut value = self.eval(host, &value, 0)?;
                if is_set {
                    if !matches!(value, Value::Object(_) | Value::Empty) {
                        return Err(Error::new(0, "Set requires an object reference"));
                    }
                } else {
                    value = scalar(host, value)?;
                }
                self.assign(host, &target, value, is_set)?;
            }
            Instruction::Evaluate(expr) => {
                if let Some(index) = self.procedure_call(&expr)? {
                    if self.frames.len() >= MAX_DEPTH {
                        return Err(Error::new(0, "Procedure recursion exceeds the limit"));
                    }
                    let frame = self.program.create_frame(index)?;
                    let retained_bytes = self.retained_bytes.saturating_add(frame.bytes);
                    if retained_bytes > MAX_RETAINED_BYTES {
                        return Err(Error::new(0, "Procedure frames exceed the 16 MiB memory limit"));
                    }
                    self.retained_bytes = retained_bytes;
                    self.frames.push(frame);
                } else {
                    self.eval(host, &expr, 0)?;
                }
            }
            Instruction::Jump(target) => self.jump(target)?,
            Instruction::JumpIfFalse(expr, target) => {
                let value = self.eval(host, &expr, 0)?;
                if !value_bool(scalar(host, value)?)? {
                    self.jump(target)?;
                }
            }
            Instruction::ForStart { name, from, to, step, exit } => {
                let kind = self.variable_type(&name)?;
                if matches!(kind, VariableType::Text | VariableType::Bool | VariableType::Object) {
                    return Err(Error::new(0, "For requires a numeric control variable"));
                }
                let from = self.eval(host, &from, 0)?;
                let from = value_number(kind.convert(scalar(host, from)?)?)?;
                let end = self.eval(host, &to, 0)?;
                let end = value_number(kind.convert(scalar(host, end)?)?)?;
                let step = self.eval(host, &step, 0)?;
                let step = value_number(kind.convert(scalar(host, step)?)?)?;
                if step == 0.0 {
                    return Err(Error::new(0, "For Step 0 is rejected to prevent an unbounded loop"));
                }
                self.assign_variable(&name, Value::Number(from))?;
                if (step > 0.0 && from > end) || (step < 0.0 && from < end) {
                    self.jump(exit)?;
                } else {
                    self.frame_mut()?.loops.push(ForState { start: pc, name, end, step, collection: None, index: 0 });
                }
            }
            Instruction::EachStart { name, collection, exit } => {
                let collection = self.eval(host, &collection, 0)?;
                let count = value_number(host.get(Some(&collection), "__each_count", &[])?)?;
                if count == 0.0 {
                    self.jump(exit)?;
                } else {
                    let value = host.get(Some(&collection), "__each_item", &[Value::Number(1.0)])?;
                    self.assign_variable(&name, value)?;
                    self.frame_mut()?.loops.push(ForState { start: pc, name, end: count, step: 0.0, collection: Some(collection), index: 1 });
                }
            }
            Instruction::EachNext { name, body } => {
                let state = self.frame()?.loops.last().ok_or_else(|| Error::new(0, "No active For loop"))?;
                if state.name != name {
                    return Err(Error::new(0, "For loop state mismatch"));
                }
                let collection = state.collection.clone().ok_or_else(|| Error::new(0, "For Each collection missing"))?;
                let next = state.index.saturating_add(1);
                if (next as f64) <= state.end {
                    let value = host.get(Some(&collection), "__each_item", &[Value::Number(next as f64)])?;
                    self.assign_variable(&name, value)?;
                    if let Some(state) = self.frame_mut()?.loops.last_mut() {
                        state.index = next;
                    }
                    self.jump(body)?;
                } else {
                    self.frame_mut()?.loops.pop();
                }
            }
            Instruction::ForNext { name, body } => {
                let state = self.frame()?.loops.last().ok_or_else(|| Error::new(0, "No active For loop"))?;
                if state.name != name {
                    return Err(Error::new(0, "For loop state mismatch"));
                }
                let end = state.end;
                let step = state.step;
                let current = self.frame()?.vars.get(&name).cloned().ok_or_else(|| Error::new(0, "For variable is missing"))?;
                let next = value_number(current)? + step;
                if !next.is_finite() {
                    return Err(Error::new(0, "For variable overflow"));
                }
                self.assign_variable(&name, Value::Number(next))?;
                if (step > 0.0 && next <= end) || (step < 0.0 && next >= end) {
                    self.jump(body)?;
                } else {
                    self.frame_mut()?.loops.pop();
                }
            }
            Instruction::LeaveLoop { target, with_depth, for_start, for_depth } => {
                let frame = self.frame_mut()?;
                frame.with.truncate(with_depth);
                if let Some(start) = for_start {
                    let at =
                        frame.loops.iter().rposition(|state| state.start == start).ok_or_else(|| Error::new(0, "No active For loop for Exit"))?;
                    frame.loops.truncate(at);
                } else {
                    frame.loops.truncate(for_depth);
                }
                self.jump(target)?;
            }
            Instruction::WithStart(expr) => {
                let value = self.eval(host, &expr, 0)?;
                if !matches!(value, Value::Object(_)) {
                    return Err(Error::new(0, "With requires an object"));
                }
                self.frame_mut()?.with.push(value);
            }
            Instruction::WithEnd => {
                if self.frame_mut()?.with.pop().is_none() {
                    return Err(Error::new(0, "No active With block"));
                }
            }
            Instruction::Return => {
                self.pop_frame();
            }
        }
        Ok(())
    }

    fn procedure_call(&self, expr: &Expr) -> Result<Option<usize>, Error> {
        let (name, args) = match expr {
            Expr::Name(name) => (name, None),
            Expr::Apply(callee, args) => match callee.as_ref() {
                Expr::Name(name) => (name, Some(args)),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let index = self.program.procedures.iter().position(|p| p.name.eq_ignore_ascii_case(name));
        if index.is_some() && args.is_some_and(|args| !args.is_empty()) {
            return Err(Error::new(0, "This Sub accepts no arguments"));
        }
        Ok(index)
    }

    fn eval(&mut self, host: &mut impl Host, expr: &Expr, depth: usize) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::new(0, "Expression evaluation exceeds the nesting limit"));
        }
        let next = depth.saturating_add(1);
        match expr {
            Expr::Literal(value) => Ok(value.clone()),
            Expr::Name(name) => {
                if let Some(value) = self.frame()?.vars.get(&key(name)) {
                    return Ok(value.clone());
                }
                match key(name).as_str() {
                    "vbcrlf" => return Ok(Value::Text("\r\n".to_string())),
                    "vbcr" => return Ok(Value::Text("\r".to_string())),
                    "vblf" => return Ok(Value::Text("\n".to_string())),
                    "vbtab" => return Ok(Value::Text("\t".to_string())),
                    "vbnullstring" => return Ok(Value::Text(String::new())),
                    _ => {}
                }
                host_get(host, None, name, &[])
            }
            Expr::Member(receiver, name) => {
                let value = self.eval(host, receiver, next)?;
                host_get(host, Some(&value), name, &[])
            }
            Expr::WithMember(name) => {
                let receiver = self.frame()?.with.last().cloned().ok_or_else(|| Error::new(0, "A leading dot requires With"))?;
                host_get(host, Some(&receiver), name, &[])
            }
            Expr::Apply(callee, args) => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval(host, arg, next)?);
                }
                match callee.as_ref() {
                    Expr::Name(name) => {
                        if let Some(receiver) = self.frame()?.vars.get(&key(name)).cloned() {
                            return host_get(host, Some(&receiver), "", &values);
                        }
                        if let Some(value) = builtin(host, name, &values)? {
                            return Ok(value);
                        }
                        host_get(host, None, name, &values)
                    }
                    Expr::Member(receiver, name) => {
                        let receiver = self.eval(host, receiver, next)?;
                        host_get(host, Some(&receiver), name, &values)
                    }
                    Expr::WithMember(name) => {
                        let receiver = self.frame()?.with.last().cloned().ok_or_else(|| Error::new(0, "A leading dot requires With"))?;
                        host_get(host, Some(&receiver), name, &values)
                    }
                    _ => {
                        let receiver = self.eval(host, callee, next)?;
                        host_get(host, Some(&receiver), "", &values)
                    }
                }
            }
            Expr::Unary(op, expr) => {
                let value = self.eval(host, expr, next)?;
                let value = scalar(host, value)?;
                match op.as_str() {
                    "+" => checked_number(value_number(value)?),
                    "-" => checked_number(-value_number(value)?),
                    "not" => match value {
                        Value::Bool(b) => Ok(Value::Bool(!b)),
                        other => Ok(Value::Number(f64::from(!integer(other)?))),
                    },
                    "condition_not" => Ok(Value::Bool(!value_bool(value)?)),
                    _ => Err(Error::new(0, "Unsupported unary operator")),
                }
            }
            Expr::Binary(op, a, b) => {
                let a = self.eval(host, a, next)?;
                let b = self.eval(host, b, next)?;
                binary(host, op, a, b)
            }
        }
    }

    fn assign(&mut self, host: &mut impl Host, target: &Expr, value: Value, is_set: bool) -> Result<(), Error> {
        match target {
            Expr::Name(name) => {
                if !is_set && let Some(receiver @ Value::Object(_)) = self.frame()?.vars.get(&key(name)).cloned() {
                    return host.set(Some(&receiver), "", &[], value);
                }
                self.assign_variable(name, value)
            }
            Expr::Member(receiver, name) => {
                let receiver = self.eval(host, receiver, 0)?;
                host.set(Some(&receiver), name, &[], value)
            }
            Expr::WithMember(name) => {
                let receiver = self.frame()?.with.last().cloned().ok_or_else(|| Error::new(0, "A leading dot requires With"))?;
                host.set(Some(&receiver), name, &[], value)
            }
            Expr::Apply(callee, args) => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval(host, arg, 0)?);
                }
                match callee.as_ref() {
                    Expr::Name(name) => {
                        if let Some(receiver) = self.frame()?.vars.get(&key(name)).cloned() {
                            host.set(Some(&receiver), "", &values, value)
                        } else {
                            host.set(None, name, &values, value)
                        }
                    }
                    Expr::Member(receiver, name) => {
                        let receiver = self.eval(host, receiver, 0)?;
                        host.set(Some(&receiver), name, &values, value)
                    }
                    Expr::WithMember(name) => {
                        let receiver = self.frame()?.with.last().cloned().ok_or_else(|| Error::new(0, "A leading dot requires With"))?;
                        host.set(Some(&receiver), name, &values, value)
                    }
                    _ => {
                        let receiver = self.eval(host, callee, 0)?;
                        host.set(Some(&receiver), "", &values, value)
                    }
                }
            }
            _ => Err(Error::new(0, "Invalid assignment target")),
        }
    }
}

fn builtin(host: &mut impl Host, name: &str, args: &[Value]) -> Result<Option<Value>, Error> {
    let name = key(name);
    if matches!(name.as_str(), "round" | "left" | "right" | "mid" | "replace" | "instr") {
        let args = args.iter().cloned().map(|arg| scalar(host, arg)).collect::<Result<Vec<_>, _>>()?;
        let at = |i: usize| args.get(i).cloned().ok_or_else(|| Error::new(0, "Missing function argument"));
        let result = match name.as_str() {
            "round" => {
                if !(1..=2).contains(&args.len()) {
                    return Err(Error::new(0, "Round requires one or two arguments"));
                }
                let n = value_number(at(0)?)?;
                let digits = if args.len() == 2 { integer(at(1)?)? } else { 0 };
                if !(0..=15).contains(&digits) {
                    return Err(Error::new(0, "Round digits must be between 0 and 15"));
                }
                let factor = 10_f64.powi(digits);
                checked_number((n * factor).round_ties_even() / factor)?
            }
            "left" | "right" | "mid" => {
                if (name == "mid" && !(2..=3).contains(&args.len())) || (name != "mid" && args.len() != 2) {
                    return Err(Error::new(0, "Invalid string function argument count"));
                }
                let text = value_text(at(0)?)?;
                let count = integer(at(1)?)?;
                if count < 0 || (name == "mid" && count == 0) {
                    return Err(Error::new(0, "Invalid string position or length"));
                }
                let chars: Vec<char> = text.chars().collect();
                let (start, len) = match name.as_str() {
                    "left" => (0, count as usize),
                    "right" => (chars.len().saturating_sub(count as usize), count as usize),
                    _ => {
                        let len = if args.len() == 3 { integer(at(2)?)? } else { i32::MAX };
                        if len < 0 {
                            return Err(Error::new(0, "Invalid string position or length"));
                        }
                        ((count - 1) as usize, len as usize)
                    }
                };
                Value::Text(chars.into_iter().skip(start).take(len).collect())
            }
            "replace" => {
                if args.len() != 3 {
                    return Err(Error::new(0, "Replace requires three arguments"));
                }
                let source = value_text(at(0)?)?;
                let find = value_text(at(1)?)?;
                let replacement = value_text(at(2)?)?;
                if find.is_empty() {
                    Value::Text(source)
                } else {
                    let count = source.matches(&find).count();
                    let size = source.len().saturating_sub(count.saturating_mul(find.len())).saturating_add(count.saturating_mul(replacement.len()));
                    if size > MAX_TEXT {
                        return Err(Error::new(0, "Text exceeds the 1 MiB limit"));
                    }
                    Value::Text(source.replace(&find, &replacement))
                }
            }
            _ => {
                if !(2..=3).contains(&args.len()) {
                    return Err(Error::new(0, "InStr requires two or three arguments"));
                }
                let (start, text_arg, pattern_arg) = if args.len() == 3 { (integer(at(0)?)?, 1, 2) } else { (1, 0, 1) };
                if start < 1 {
                    return Err(Error::new(0, "Invalid string position or length"));
                }
                let source = value_text(at(text_arg)?)?;
                let pattern = value_text(at(pattern_arg)?)?;
                let char_start = (start - 1) as usize;
                let offset =
                    source.char_indices().nth(char_start).map(|(i, _)| i).or_else(|| (char_start == source.chars().count()).then_some(source.len()));
                let found = offset.and_then(|offset| source.get(offset..).and_then(|s| s.find(&pattern)).map(|i| offset + i));
                Value::Number(found.map_or(0.0, |i| source[..i].chars().count() as f64 + 1.0))
            }
        };
        return Ok(Some(result));
    }
    if !matches!(name.as_str(), "cstr" | "cdbl" | "cint" | "clng" | "len" | "ucase" | "lcase" | "trim" | "abs" | "int" | "fix" | "sqr" | "cbool") {
        return Ok(None);
    }
    if args.len() != 1 {
        return Err(Error::new(0, format!("{name} requires one argument")));
    }
    let arg = args.first().cloned().ok_or_else(|| Error::new(0, "Missing function argument"))?;
    let arg = scalar(host, arg)?;
    let result = match name.as_str() {
        "cstr" => Value::Text(value_text(arg)?),
        "cdbl" => Value::Number(value_number(arg)?),
        "cbool" => Value::Bool(value_bool(arg)?),
        "cint" | "clng" => {
            let n = integer(arg)?;
            if name == "cint" && !(i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&n) {
                return Err(Error::new(0, "Integer overflow"));
            }
            Value::Number(f64::from(n))
        }
        "len" => Value::Number(value_text(arg)?.encode_utf16().count() as f64),
        "ucase" | "lcase" | "trim" => {
            let text = value_text(arg)?;
            let text = match name.as_str() {
                "ucase" => text.to_uppercase(),
                "lcase" => text.to_lowercase(),
                _ => text.trim_matches(' ').to_string(),
            };
            if text.len() > MAX_TEXT {
                return Err(Error::new(0, "Text exceeds the 1 MiB limit"));
            }
            Value::Text(text)
        }
        "abs" => checked_number(value_number(arg)?.abs())?,
        "int" => checked_number(value_number(arg)?.floor())?,
        "fix" => checked_number(value_number(arg)?.trunc())?,
        "sqr" => {
            let number = value_number(arg)?;
            if number < 0.0 {
                return Err(Error::new(0, "Invalid procedure call: Sqr argument is negative"));
            }
            checked_number(number.sqrt())?
        }
        _ => return Err(Error::new(0, "Unknown built-in function")),
    };
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockHost {
        values: BTreeMap<String, Value>,
        log: Vec<Value>,
    }

    impl Host for MockHost {
        fn get(&mut self, receiver: Option<&Value>, name: &str, args: &[Value]) -> Result<Value, Error> {
            match (receiver, key(name).as_str()) {
                (None, "range") => Ok(Value::Object(1)),
                (None, "cells") => Ok(Value::Object(2)),
                (None, "debug") => Ok(Value::Object(3)),
                (Some(Value::Object(1)), "" | "value") => Ok(self.values.get("a1").cloned().unwrap_or(Value::Empty)),
                (Some(Value::Object(3)), "print") => {
                    self.log.extend_from_slice(args);
                    Ok(Value::Empty)
                }
                (None, "inspect") => {
                    self.log.extend_from_slice(args);
                    Ok(Value::Empty)
                }
                _ => Err(Error::new(0, format!("Unknown member: {name}"))),
            }
        }

        fn set(&mut self, receiver: Option<&Value>, name: &str, _args: &[Value], value: Value) -> Result<(), Error> {
            if matches!(receiver, Some(Value::Object(1))) && (name.is_empty() || name.eq_ignore_ascii_case("value")) {
                self.values.insert("a1".to_string(), value);
                Ok(())
            } else {
                Err(Error::new(0, "Unknown property"))
            }
        }
    }

    fn run(source: &str) -> Result<MockHost, Error> {
        let program = Program::compile(source)?;
        let mut execution = program.start("Main")?;
        let mut host = MockHost::default();
        for _ in 0..1000 {
            if execution.step(&mut host, 100)? {
                return Ok(host);
            }
        }
        Err(Error::new(execution.line(), "Test exceeded its execution budget"))
    }

    #[test]
    fn loops_conditions_comments_and_case_insensitivity() -> Result<(), Error> {
        let host = run(
            "Option Explicit\nSub MAIN()\nDim i As Long, total As Double\nFor i = 1 To 5\nIf i Mod 2 = 0 Then\ntotal = total + i\nElseIf i = 5 Then\ntotal = total + 10\nElse\ntotal = total + 1\nEnd If\nNext I\nDebug.Print total ' comment\nEnd Sub",
        )?;
        assert_eq!(host.log, vec![Value::Number(18.0)]);
        Ok(())
    }

    #[test]
    fn object_set_default_value_with_and_argument_identity() -> Result<(), Error> {
        let host = run(
            "Sub Main()\nDim r As Range, n As Double\nSet r = Range(\"A1\")\nWith r\n.Value = 41\nEnd With\nn = r + 1\nInspect r\nDebug.Print n\nEnd Sub",
        )?;
        assert_eq!(host.log, vec![Value::Object(1), Value::Number(42.0)]);
        Ok(())
    }

    #[test]
    fn do_while_wend_exit_and_sub_calls() -> Result<(), Error> {
        let host = run(
            "Sub Helper()\nDebug.Print \"ok\"\nEnd Sub\nSub Main()\nDim i\nDo\ni = i + 1\nIf i = 3 Then Exit Do\nLoop\nWhile i < 5\ni = i + 1\nWend\nDo Until i = 7\ni = i + 1\nLoop\nFor i = 9 To 1 Step -1\nIf i = 6 Then Exit For\nNext i\nCall Helper()\nDebug.Print i\nEnd Sub",
        )?;
        assert_eq!(host.log, vec![Value::Text("ok".to_string()), Value::Number(6.0)]);
        Ok(())
    }

    #[test]
    fn runtime_errors_report_original_source_line_and_stay_failed() -> Result<(), Error> {
        let program = Program::compile("Sub Main()\nDim a\na = 1 / 0\nEnd Sub")?;
        let mut execution = program.start("Main")?;
        let mut host = MockHost::default();
        let error = execution.step(&mut host, 100).err().ok_or_else(|| Error::new(0, "Expected an error"))?;
        assert_eq!(error.line, 3);
        assert!(error.message.contains("zero"));
        assert!(execution.step(&mut host, 100).is_err());
        Ok(())
    }

    #[test]
    fn execution_yields_for_infinite_loop_and_zero_budget() -> Result<(), Error> {
        let program = Program::compile("Sub Main()\nDo\nLoop\nEnd Sub")?;
        let mut execution = program.start("Main")?;
        let mut host = MockHost::default();
        assert!(!execution.step(&mut host, 0)?);
        assert_eq!(execution.steps(), 0);
        assert!(!execution.step(&mut host, 17)?);
        assert_eq!(execution.steps(), 17);
        Ok(())
    }

    #[test]
    fn unsupported_syntax_and_deep_inputs_fail_without_execution() {
        for source in ["Sub Main()\nOn Error Resume Next\nEnd Sub", "Sub Main(x As Long)\nEnd Sub", "Sub Main()\nSelect Case x\nEnd Select\nEnd Sub"]
        {
            assert!(Program::compile(source).is_err());
        }
        let source = format!("Sub Main()\na = {}1{}\nEnd Sub", "(".repeat(1000), ")".repeat(1000));
        assert!(Program::compile(&source).is_err());
        let source = format!("Sub Main()\na = {}1\nEnd Sub", "1+".repeat(1000));
        assert!(Program::compile(&source).is_err());
        let mut expression = "1".to_string();
        for _ in 0..40 {
            expression = format!("({expression})+1+1+1");
        }
        let source = format!("Sub Main()\na = {expression}\nEnd Sub");
        assert!(Program::compile(&source).is_err());
    }

    #[test]
    fn strings_continuations_and_inline_if() -> Result<(), Error> {
        let host = run("Sub Main()\nDim a\na = \"a:'\" & _\n\"b\"\nIf 2 ^ 3 = 8 Then Debug.Print a Else Debug.Print \"bad\"\nEnd Sub")?;
        assert_eq!(host.log, vec![Value::Text("a:'b".to_string())]);
        Ok(())
    }

    #[test]
    fn option_explicit_rejects_undeclared_assignment() -> Result<(), Error> {
        let program = Program::compile("Option Explicit\nSub Main()\nmissing = 1\nEnd Sub")?;
        let mut execution = program.start("Main")?;
        let error = execution.step(&mut MockHost::default(), 100).err().ok_or_else(|| Error::new(0, "Expected an error"))?;
        assert_eq!(error.line, 3);
        assert!(error.message.contains("defined"));
        let error = run("Option Explicit\nSub Main()\nDim a\na = missing\nEnd Sub")
            .err()
            .ok_or_else(|| Error::new(0, "Expected an undeclared read to fail"))?;
        assert_eq!(error.line, 4);
        Ok(())
    }

    #[test]
    fn dim_is_procedure_scoped_and_integer_assignment_rounds_to_even() -> Result<(), Error> {
        let host = run(
            "Option Explicit\nSub Main()\nIf False Then\nDim n As Long\nEnd If\nn = 2.5\nDebug.Print n\nFor n = 1 To 2\nDim a As Integer\na = a + 1\nNext n\nDebug.Print a\nEnd Sub",
        )?;
        assert_eq!(host.log, vec![Value::Number(2.0), Value::Number(2.0)]);
        Ok(())
    }

    #[test]
    fn numeric_loop_conditions_use_boolean_conversion_and_exit_cleans_scopes() -> Result<(), Error> {
        let host = run(
            "Sub Main()\nDim i, j\nDo Until 1\nDebug.Print \"bad\"\nLoop\nFor i = 1 To 2\nDo\nFor j = 1 To 3\nWith Range(\"A1\")\nExit Do\nEnd With\nNext j\nLoop\nNext i\nDebug.Print i = 3\nEnd Sub",
        )?;
        assert_eq!(host.log, vec![Value::Bool(true)]);
        Ok(())
    }

    #[test]
    fn known_exported_attributes_and_rem_comments_are_not_executed() -> Result<(), Error> {
        let host = run(
            "Attribute VB_Name = \"Module1\"\nAttribute VB_Exposed = False\nSub Main()\nRem \"unmatched quote is a comment\nDim n: Rem comment: UnknownCall\nDebug.Print \"ok\"\nEnd Sub",
        )?;
        assert_eq!(host.log, vec![Value::Text("ok".to_string())]);
        assert!(Program::compile("Attribute Unknown = 1\nSub Main()\nEnd Sub").is_err());
        assert!(Program::compile("Sub Main()\nIf False Then a = 1: Debug.Print \"bad\"\nEnd Sub").is_err());
        Ok(())
    }

    #[test]
    fn object_variables_assign_through_the_default_property_without_set() -> Result<(), Error> {
        let host = run("Sub Main()\nDim r As Object\nSet r = Range(\"A1\")\nr = 9\nInspect r\nDebug.Print r + 1\nEnd Sub")?;
        assert_eq!(host.log, vec![Value::Object(1), Value::Number(10.0)]);
        Ok(())
    }

    #[test]
    fn not_comparison_precedence_and_invalid_set_are_checked() -> Result<(), Error> {
        let host = run("Sub Main()\nDebug.Print Not 1 = 2\nDebug.Print Not 0\nDebug.Print -2 ^ 2\nEnd Sub")?;
        assert_eq!(host.log, vec![Value::Bool(true), Value::Number(-1.0), Value::Number(-4.0)]);
        let error = run("Sub Main()\nDim r\nSet r = 123\nEnd Sub").err().ok_or_else(|| Error::new(0, "Expected invalid Set to fail"))?;
        assert_eq!(error.line, 3);
        Ok(())
    }

    #[test]
    fn call_stack_and_variable_memory_are_bounded() -> Result<(), Error> {
        let program = Program::compile("Sub Main()\nCall Main()\nEnd Sub")?;
        let mut execution = program.start("Main")?;
        assert!(execution.step(&mut MockHost::default(), 100).is_err());
        let mut source = "Sub Main()\nDim text\ntext = \"x\"\n".to_string();
        for _ in 0..20 {
            source.push_str("text = text & text\n");
        }
        for index in 0..20 {
            source.push_str(&format!("value{index} = text\n"));
        }
        source.push_str("End Sub");
        let error = run(&source).err().ok_or_else(|| Error::new(0, "Expected memory limit to fail"))?;
        assert!(error.message.contains("memory limit"));
        Ok(())
    }

    #[test]
    fn hostile_host_arrays_are_rejected_without_recursive_drop() -> Result<(), Error> {
        struct NestedHost;
        impl Host for NestedHost {
            fn get(&mut self, _receiver: Option<&Value>, _name: &str, _args: &[Value]) -> Result<Value, Error> {
                let mut value = Value::Empty;
                for _ in 0..2000 {
                    value = Value::Array(vec![vec![value]]);
                }
                Ok(value)
            }
            fn set(&mut self, _receiver: Option<&Value>, _name: &str, _args: &[Value], _value: Value) -> Result<(), Error> {
                Err(Error::new(0, "Unused"))
            }
        }
        let program = Program::compile("Sub Main()\nDim a\na = UnknownValue\nEnd Sub")?;
        let mut execution = program.start("Main")?;
        let error = execution.step(&mut NestedHost, 100).err().ok_or_else(|| Error::new(0, "Expected array limit to fail"))?;
        assert_eq!(error.line, 3);
        assert!(error.message.contains("nesting"));
        Ok(())
    }
}
