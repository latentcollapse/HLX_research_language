//! HLX-A (ASCII) Parser and Emitter

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, multispace1, digit1, one_of},
    combinator::{opt, map, value, cut},
    multi::{many0, separated_list0},
    sequence::{tuple, delimited, preceded, terminated},
    error::{VerboseError, context},
};
use tracing::{instrument, debug, trace};

use crate::ast::*;
use crate::parser::Parser;
use crate::emitter::Emitter;
use hlx_core::{Result, HlxError};

type ParseResult<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

/// Helper to compute span from input positions
fn compute_span(original: &str, start_input: &str, end_input: &str) -> Span {
    let start_offset = original.len() - start_input.len();
    let end_offset = original.len() - end_input.len();

    // Compute line and column for start position
    let prefix = &original[..start_offset];
    let mut line = 1u32;
    let mut col = 1u32;

    for ch in prefix.chars() {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    Span::new(start_offset, end_offset, line, col)
}

/// Combinator that wraps a parser and produces a Spanned result
/// Requires the original input to compute absolute positions
/// Skips leading whitespace before capturing the span to get accurate line numbers
fn spanned<'a, O, F>(
    original: &'a str,
    mut parser: F,
) -> impl FnMut(&'a str) -> ParseResult<'a, Spanned<O>>
where
    F: FnMut(&'a str) -> ParseResult<'a, O>,
{
    move |input: &'a str| {
        // Skip leading whitespace to capture the actual token position
        let (after_ws, _) = ws(input)?;
        let start_input = after_ws;
        let (remaining, output) = parser(input)?;
        let span = compute_span(original, start_input, remaining);
        Ok((remaining, Spanned::new(output, span)))
    }
}

fn ws(input: &str) -> ParseResult<'_, ()> {
    let (input, _) = many0(alt((
        value((), multispace1),
        value((), tuple((tag("//"), take_while(|c| c != '\n'), opt(char('\n'))))),
    )))(input)?;
    Ok((input, ()))
}

fn is_ident_start(c: char) -> bool { c.is_alphabetic() || c == '_' }
fn is_ident_cont(c: char) -> bool { c.is_alphanumeric() || c == '_' }

fn ident(input: &str) -> ParseResult<'_, String> {
    let (input, first) = take_while1(is_ident_start)(input)?;
    let (input, rest) = take_while(is_ident_cont)(input)?;
    let id = format!("{}{}", first, rest);
    let keywords = ["let", "return", "if", "else", "loop", "program", "block", "fn", "null", "true", "false", "and", "or", "not", "break", "continue"];
    if keywords.contains(&id.as_str()) {
        return Err(nom::Err::Error(VerboseError {
            errors: vec![(input, nom::error::VerboseErrorKind::Context("keyword"))]
        }));
    }
    Ok((input, id))
}

fn parse_type(input: &str) -> ParseResult<'_, Type> {
    // Try to parse Array<T> first
    if let Ok((input, _)) = tag::<_, _, VerboseError<&str>>("Array")(input) {
        let (input, _) = preceded(ws, char('<'))(input)?;
        let (input, inner) = preceded(ws, parse_type)(input)?;
        let (input, _) = preceded(ws, char('>'))(input)?;
        return Ok((input, Type::Array(Box::new(inner))));
    }

    // Try primitive types first, then fall back to named types
    alt((
        map(alt((tag("Int"), tag("int"))), |_| Type::Int),
        map(alt((tag("Float"), tag("float"))), |_| Type::Float),
        map(alt((tag("String"), tag("string"))), |_| Type::String),
        map(alt((tag("Bool"), tag("bool"))), |_| Type::Bool),
        // Fall back to any identifier as a named type (e.g., "object", "tensor_t")
        map(ident, |name| Type::Named(name)),
    ))(input)
}

fn parse_string_literal(input: &str) -> ParseResult<'_, String> {
    delimited(
        char('"'),
        map(many0(alt((
            map(take_while1(|c| c != '"' && c != '\\'), |s: &str| s.to_string()),
            map(preceded(char('\\'), one_of("\"\\/bfnrt")), |c| match c {
                '"' => "\"",
                '\\' => "\\",
                '/' => "/",
                'b' => "\x08",
                'f' => "\x0c",
                'n' => "\n",
                'r' => "\r",
                't' => "\t",
                _ => unreachable!(),
            }.to_string())
        ))), |fragments| fragments.concat()),
        char('"')
    )(input)
}

fn literal(input: &str) -> ParseResult<'_, Literal> {
    alt((
        value(Literal::Null, tag("null")),
        value(Literal::Bool(true), tag("true")),
        value(Literal::Bool(false), tag("false")),
        map(tuple((opt(char('-')), digit1, char('.'), digit1)), |(s, w, _, f)| {
            let n = format!("{}{}.{}", s.unwrap_or(' '), w, f);
            Literal::Float(n.trim().parse().unwrap())
        }),
        map(tuple((opt(char('-')), digit1)), |(s, d)| {
            let n = format!("{}{}", s.unwrap_or(' '), d);
            Literal::Int(n.trim().parse().unwrap())
        }),
        map(parse_string_literal, Literal::String),
    ))(input)
}

fn array_literal<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Expr> {
    let (input, elems) = delimited(
        preceded(ws, char('[')),
        separated_list0(preceded(ws, char(',')), |i| spanned(original, |i2| expr(original, i2))(i)),
        preceded(ws, char(']'))
    )(input)?;

    Ok((input, Expr::Array(elems)))
}

fn object_literal<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Expr> {
    let (input, items) = delimited(
        preceded(ws, char('{')),
        separated_list0(preceded(ws, char(',')),
            |i| {
                let (i, key) = preceded(ws, alt((
                    map(delimited(char('"'), take_while(|c| c != '"'), char('"')), |s: &str| s.to_string()),
                    map(delimited(char('\''), take_while(|c| c != '\''), char('\'')), |s: &str| s.to_string()),
                    ident
                )))(i)?;
                let (i, _) = preceded(ws, char(':'))(i)?;
                let (i, value) = spanned(original, |i2| expr(original, i2))(i)?;
                Ok((i, (key, value)))
            }
        ),
        preceded(ws, char('}'))
    )(input)?;

    Ok((input, Expr::Object(items)))
}

fn call_expr<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Expr> {
    let start_input = input;
    let (input, name) = preceded(ws, ident)(input)?;
    let name_span = compute_span(original, start_input, input);

    let (input, args) = delimited(
        preceded(ws, char('(')),
        separated_list0(preceded(ws, char(',')), |i| spanned(original, |i2| expr(original, i2))(i)),
        preceded(ws, char(')'))
    )(input)?;

    Ok((input, Expr::Call {
        func: Box::new(Spanned::new(Expr::Ident(name), name_span)),
        args
    }))
}

enum Postfix {
    Index(Expr),
    Field(String),
}

fn atom_expr<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Expr> {
    let (input, atom) = alt((
        map(preceded(ws, literal), Expr::Literal),
        |i| array_literal(original, i),
        |i| object_literal(original, i),
        |i| call_expr(original, i),
        map(preceded(ws, ident), Expr::Ident),
        delimited(preceded(ws, char('(')), |i| expr(original, i), preceded(ws, char(')'))),
    ))(input)?;

    let (input, postfixes) = many0(alt((
        map(delimited(
            preceded(ws, char('[')),
            |i| expr(original, i),
            preceded(ws, char(']'))
        ), Postfix::Index),
        map(preceded(
            preceded(ws, char('.')),
            ident
        ), Postfix::Field)
    )))(input)?;

    // Build spans for accumulated expressions
    // Note: Postfix operations are challenging to track precisely since they're folded
    // after parsing. For now we use placeholder spans for the accumulated objects.
    let (input, final_expr) = postfixes.into_iter().fold((input, atom), |(remaining, acc), op| {
        let acc_span = Span::new(0, 0, 0, 0); // Placeholder span for accumulated expression
        match op {
            Postfix::Index(idx) => {
                let idx_span = Span::new(0, 0, 0, 0); // Placeholder span for index expression
                (remaining, Expr::Index {
                    object: Box::new(Spanned::new(acc, acc_span)),
                    index: Box::new(Spanned::new(idx, idx_span))
                })
            },
            Postfix::Field(field) => {
                (remaining, Expr::Field {
                    object: Box::new(Spanned::new(acc, acc_span)),
                    field
                })
            }
        }
    });

    Ok((input, final_expr))
}

fn unary_expr<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Expr> {
    alt((
        |i| {
            let (i, _) = preceded(ws, char('-'))(i)?;
            let (i, operand) = spanned(original, |i2| atom_expr(original, i2))(i)?;
            Ok((i, Expr::UnaryOp { op: UnaryOp::Neg, operand: Box::new(operand) }))
        },
        |i| {
            let (i, _) = preceded(ws, alt((tag("not"), tag("!"))))(i)?;
            let (i, operand) = spanned(original, |i2| atom_expr(original, i2))(i)?;
            Ok((i, Expr::UnaryOp { op: UnaryOp::Not, operand: Box::new(operand) }))
        },
        |i| atom_expr(original, i)
    ))(input)
}

fn bin_op(input: &str) -> ParseResult<'_, BinOp> {
    preceded(ws, alt((
        value(BinOp::Add, char('+')),
        value(BinOp::Sub, char('-')),
        value(BinOp::Mul, char('*')),
        value(BinOp::Div, char('/')),
        value(BinOp::Mod, char('%')),
        value(BinOp::Eq, tag("==")),
        value(BinOp::Ne, tag("!=")),
        value(BinOp::Le, tag("<=")),
        value(BinOp::Ge, tag(">=")),
        value(BinOp::Lt, char('<')),
        value(BinOp::Gt, char('>')),
        value(BinOp::And, tag("and")),
        value(BinOp::Or, tag("or")),
    )))(input)
}

fn expr<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Expr> {
    let start_input = input;
    let (input, lhs) = unary_expr(original, input)?;
    let lhs_span = compute_span(original, start_input, input);
    let (input, op_opt) = opt(bin_op)(input)?;

    if let Some(op) = op_opt {
        let start_rhs = input;
        let (input, rhs) = expr(original, input)?;
        let rhs_span = compute_span(original, start_rhs, input);
        // Enforce no operator precedence: if RHS is a BinOp, it MUST be parenthesized
        // (Our recursive call to expr will handle nested BinOps, but we want to
        // discourage this in the future or enforce it via the parser's structure)
        Ok((input, Expr::BinOp {
            op,
            lhs: Box::new(Spanned::new(lhs, lhs_span)),
            rhs: Box::new(Spanned::new(rhs, rhs_span))
        }))
    } else {
        Ok((input, lhs))
    }
}

fn statement<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Statement> {
    alt((
        // let name = value; or let name: Type = value;
        context("let statement",
            |i| {
                let (i, _) = preceded(ws, tag("let"))(i)?;
                let (i, n) = cut(context("variable name", preceded(ws, ident)))(i)?;
                let (i, t) = opt(preceded(preceded(ws, char(':')), preceded(ws, parse_type)))(i)?;
                let (i, _) = cut(context("'=' after variable name", preceded(ws, char('='))))(i)?;
                let (i, v) = cut(context("expression", |i2| spanned(original, |i3| expr(original, i3))(i2)))(i)?;
                let (i, _) = cut(context("';' after let statement", preceded(ws, char(';'))))(i)?;
                Ok((i, Statement::Let { name: n, type_annotation: t, value: v }))
            }
        ),

        // return value;
        context("return statement",
            |i| {
                let (i, _) = preceded(ws, tag("return"))(i)?;
                let (i, v) = cut(context("expression", |i2| spanned(original, |i3| expr(original, i3))(i2)))(i)?;
                let (i, _) = cut(context("';' after return", preceded(ws, char(';'))))(i)?;
                Ok((i, Statement::Return { value: v }))
            }
        ),

        // if condition { ... } else { ... }
        context("if statement",
            |i| {
                let (i, _) = preceded(ws, tag("if"))(i)?;
                let (i, cond) = cut(context("condition", |i2| {
                    spanned(original, |i3| {
                        alt((
                            delimited(preceded(ws, char('(')), |i4| expr(original, i4), preceded(ws, char(')'))),
                            |i4| expr(original, i4)
                        ))(i3)
                    })(i2)
                }))(i)?;
                let (i, _) = cut(context("'{' after if condition", preceded(ws, char('{'))))(i)?;
                let (i, then_body) = many0(|i2| spanned(original, |i3| statement(original, i3))(i2))(i)?;
                let (i, _) = cut(context("'}' to close if block", preceded(ws, char('}'))))(i)?;
                let (i, els) = opt(|i2| {
                    let (i2, _) = preceded(ws, tag("else"))(i2)?;
                    let (i2, _) = preceded(ws, char('{'))(i2)?;
                    let (i2, body) = many0(|i3| spanned(original, |i4| statement(original, i4))(i3))(i2)?;
                    let (i2, _) = preceded(ws, char('}'))(i2)?;
                    Ok((i2, body))
                })(i)?;
                Ok((i, Statement::If { condition: cond, then_branch: then_body, else_branch: els }))
            }
        ),

        // loop(condition, max_iter) { ... }
        context("loop statement",
            |i| {
                let (i, _) = preceded(ws, tag("loop"))(i)?;
                let (i, _) = cut(context("'(' after loop", preceded(ws, char('('))))(i)?;
                let (i, cond) = cut(context("loop condition", |i2| spanned(original, |i3| expr(original, i3))(i2)))(i)?;
                let (i, _) = cut(context("',' after condition", preceded(ws, char(','))))(i)?;
                let (i, max_iter) = alt((
                    map(preceded(ws, digit1), |s: &str| s.parse::<u32>().unwrap()),
                    value(1000000, preceded(ws, tag("DEFAULT_MAX_ITER()")))
                ))(i)?;
                let (i, _) = cut(context("')' after max iterations", preceded(ws, char(')'))))(i)?;
                let (i, _) = cut(context("'{' after loop header", preceded(ws, char('{'))))(i)?;
                let (i, body) = many0(|i2| spanned(original, |i3| statement(original, i3))(i2))(i)?;
                let (i, _) = cut(context("'}' to close loop", preceded(ws, char('}'))))(i)?;
                Ok((i, Statement::While { condition: cond, body, max_iter }))
            }
        ),

        // break;
        context("break statement",
            |i| {
                let (i, _) = preceded(ws, tag("break"))(i)?;
                let (i, _) = cut(context("';' after break", preceded(ws, char(';'))))(i)?;
                Ok((i, Statement::Break))
            }
        ),

        // continue;
        context("continue statement",
            |i| {
                let (i, _) = preceded(ws, tag("continue"))(i)?;
                let (i, _) = cut(context("';' after continue", preceded(ws, char(';'))))(i)?;
                Ok((i, Statement::Continue))
            }
        ),

        // assignment: lhs = value;
        context("assignment",
            |i| {
                let (i, lhs) = preceded(ws, |i2| spanned(original, |i3| atom_expr(original, i3))(i2))(i)?;
                let (i, _) = preceded(ws, char('='))(i)?;
                let (i, v) = cut(context("expression", |i2| spanned(original, |i3| expr(original, i3))(i2)))(i)?;
                let (i, _) = cut(context("';' after assignment", preceded(ws, char(';'))))(i)?;
                Ok((i, Statement::Assign { lhs, value: v }))
            }
        ),

        // expression statement
        context("expression statement",
            |i| {
                let (i, e) = preceded(ws, |i2| spanned(original, |i3| expr(original, i3))(i2))(i)?;
                let (i, _) = cut(context("';' after expression", preceded(ws, char(';'))))(i)?;
                Ok((i, Statement::Expr(e)))
            }
        ),
    ))(input)
}

fn block<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Block> {
    alt((
        // fn name(params) -> return_type { body }
        context("function definition",
            |i| {
                let (i, _) = preceded(ws, tag("fn"))(i)?;
                let (i, name) = cut(context("function name", preceded(ws, ident)))(i)?;
                let (i, _) = cut(context("'(' for parameters", preceded(ws, char('('))))(i)?;
                let (i, params) = separated_list0(preceded(ws, char(',')),
                    tuple((
                        preceded(ws, ident),
                        opt(preceded(preceded(ws, char(':')), preceded(ws, parse_type)))
                    ))
                )(i)?;
                let (i, _) = cut(context("')' after parameters", preceded(ws, char(')'))))(i)?;
                let (i, return_type) = opt(preceded(preceded(ws, tag("->")), preceded(ws, parse_type)))(i)?;
                let (i, _) = cut(context("'{' to start function body", preceded(ws, char('{'))))(i)?;
                let (i, body) = many0(|i2| spanned(original, |i3| statement(original, i3))(i2))(i)?;
                let (i, _) = cut(context("'}' to close function", preceded(ws, char('}'))))(i)?;
                let items = body.into_iter().map(|s| Spanned::new(Item::Statement(s.node), s.span)).collect();
                Ok((i, Block { name, params, return_type, items }))
            }
        ),
        // block name() { body }
        context("block definition",
            |i| {
                let (i, _) = preceded(ws, tag("block"))(i)?;
                let (i, name) = cut(context("block name", preceded(ws, ident)))(i)?;
                let (i, _) = preceded(ws, char('('))(i)?;
                let (i, _) = preceded(ws, char(')'))(i)?;
                let (i, _) = cut(context("'{' to start block body", preceded(ws, char('{'))))(i)?;
                let (i, body) = many0(|i2| spanned(original, |i3| statement(original, i3))(i2))(i)?;
                let (i, _) = cut(context("'}' to close block", preceded(ws, char('}'))))(i)?;
                let items = body.into_iter().map(|s| Spanned::new(Item::Statement(s.node), s.span)).collect();
                Ok((i, Block { name, params: vec![], return_type: None, items }))
            }
        ),
    ))(input)
}

fn parse_program(input: &str) -> ParseResult<'_, Program> {
    let original = input; // Save original for position tracking
    let (input, _) = context("'program' keyword", preceded(ws, tag("program")))(input)?;
    let (input, name) = cut(context("program name", preceded(ws, ident)))(input)?;
    let (input, _) = cut(context("'{' to start program", preceded(ws, char('{'))))(input)?;
    let (input, blocks) = many0(preceded(ws, |i| block(original, i)))(input)?;
    let (input, _) = cut(context("'}' to close program", preceded(ws, char('}'))))(input)?;
    Ok((input, Program { name, blocks }))
}

pub struct HlxaParser;
impl HlxaParser {
    pub fn new() -> Self { Self }

    pub fn parse_diagnostics(&self, source: &str) -> std::result::Result<Program, Vec<(String, usize)>> {
        match parse_program(source) {
            Ok((_, p)) => Ok(p),
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                let mut diags = Vec::new();
                // Get the primary error (first one usually points to location)
                if let Some((substring, kind)) = e.errors.first() {
                    let offset = substring.as_ptr() as usize - source.as_ptr() as usize;
                    diags.push((format!("{:?}", kind), offset));
                } else {
                     diags.push(("Unknown syntax error".to_string(), 0));
                }
                Err(diags)
            },
            Err(nom::Err::Incomplete(_)) => Err(vec![("Incomplete input".to_string(), source.len())]),
        }
    }
}
impl Parser for HlxaParser {
    fn parse(&self, source: &str) -> Result<Program> {
        match parse_program(source) {
            Ok((_, p)) => Ok(p),
            Err(e) => {
                // Use nom's convert_error for human-readable error messages
                let error_msg = match e {
                    nom::Err::Error(ve) | nom::Err::Failure(ve) => {
                        nom::error::convert_error(source, ve)
                    }
                    nom::Err::Incomplete(_) => {
                        "Incomplete input (streaming parser error)".to_string()
                    }
                };
                Err(HlxError::parse(error_msg))
            }
        }
    }
    fn name(&self) -> &'static str { "HLX-A" }
}

pub struct HlxaEmitter;
impl HlxaEmitter {
    pub fn new() -> Self { Self }

    fn emit_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Null => "null".to_string(),
                Literal::Bool(b) => b.to_string(),
                Literal::Int(n) => n.to_string(),
                Literal::Float(f) => f.to_string(),
                Literal::String(s) => format!("\"{}\"", s),
                Literal::Array(items) => {
                    let inner = items.iter()
                        .map(|item| self.emit_expr(&item.node))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("[{}]", inner)
                }
                Literal::Object(_) => "{}".to_string(), // Simplified for now
            },
            Expr::Ident(name) => name.clone(),
            Expr::BinOp { op, lhs, rhs } => {
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Mod => "%",
                    BinOp::Eq => "==",
                    BinOp::Ne => "!=",
                    BinOp::Lt => "<",
                    BinOp::Le => "<=",
                    BinOp::Gt => ">",
                    BinOp::Ge => ">=",
                    BinOp::And => "and",
                    BinOp::Or => "or",
                };
                format!("{} {} {}", self.emit_expr(&lhs.node), op_str, self.emit_expr(&rhs.node))
            },
            Expr::UnaryOp { op, operand } => {
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                };
                format!("{}{}", op_str, self.emit_expr(&operand.node))
            },
            Expr::Call { func, args } => {
                let args_str = args.iter()
                    .map(|arg| self.emit_expr(&arg.node))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", self.emit_expr(&func.node), args_str)
            },
            Expr::Index { object, index } => {
                format!("{}[{}]", self.emit_expr(&object.node), self.emit_expr(&index.node))
            },
            _ => "/* complex expr */".to_string(),
        }
    }

    fn emit_statement(&self, stmt: &Statement, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        match stmt {
            Statement::Let { name, type_annotation, value } => {
                if let Some(typ) = type_annotation {
                    format!("{}let {}: {} = {};", ind, name, typ.to_string(), self.emit_expr(&value.node))
                } else {
                    format!("{}let {} = {};", ind, name, self.emit_expr(&value.node))
                }
            },
            Statement::Assign { lhs, value } => {
                format!("{}{} = {};", ind, self.emit_expr(&lhs.node), self.emit_expr(&value.node))
            },
            Statement::Return { value } => {
                format!("{}return {};", ind, self.emit_expr(&value.node))
            },
            Statement::If { condition, then_branch, else_branch } => {
                let mut result = format!("{}if ({}) {{\n", ind, self.emit_expr(&condition.node));
                for stmt in then_branch {
                    result.push_str(&self.emit_statement(&stmt.node, indent + 1));
                    result.push('\n');
                }
                result.push_str(&format!("{}}}", ind));
                if let Some(else_stmts) = else_branch {
                    result.push_str(" else {\n");
                    for stmt in else_stmts {
                        result.push_str(&self.emit_statement(&stmt.node, indent + 1));
                        result.push('\n');
                    }
                    result.push_str(&format!("{}}}", ind));
                }
                result
            },
            Statement::While { condition, body, max_iter } => {
                let mut result = format!("{}loop({}, {}) {{\n", ind, self.emit_expr(&condition.node), max_iter);
                for stmt in body {
                    result.push_str(&self.emit_statement(&stmt.node, indent + 1));
                    result.push('\n');
                }
                result.push_str(&format!("{}}}", ind));
                result
            },
            Statement::Expr(expr) => {
                format!("{}{};", ind, self.emit_expr(&expr.node))
            },
            _ => {
                format!("{}/* unsupported statement */;", ind)
            },
        }
    }
}

impl Emitter for HlxaEmitter {
    fn emit(&self, program: &Program) -> Result<String> {
        let mut output = String::new();
        output.push_str(&format!("program {} {{\n", program.name));

        for block in &program.blocks {
            let params_str = block.params.iter()
                .map(|(name, typ)| {
                    if let Some(t) = typ {
                        format!("{}: {}", name, t.to_string())
                    } else {
                        name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
                
            let return_type = block.return_type.as_ref().map(|t| format!(" -> {}", t.to_string())).unwrap_or_default();
            output.push_str(&format!("    fn {}({}){} {{\n", block.name, params_str, return_type));

            for item in &block.items {
                match &item.node {
                    Item::Statement(stmt) => {
                        output.push_str(&self.emit_statement(stmt, 2));
                        output.push('\n');
                    }
                    Item::Node(_node) => {
                        // TODO: Implement node emission for HLX-R topological form
                        output.push_str("        /* node */\n");
                    }
                }
            }

            output.push_str("    }\n");
        }

        output.push_str("}\n");
        Ok(output)
    }
    fn name(&self) -> &'static str { "HLX-A" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diagnostics() {
        let parser = HlxaParser::new();
        let source = "program test { fn main() { print(1 } }"; // Missing closing paren
        let result = parser.parse_diagnostics(source);
        
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        let (msg, offset) = &errors[0];
        println!("Error: {} at offset {}", msg, offset);
        
        // Offset should point somewhere after 'print(1'
        assert!(*offset > 0);
    }
}