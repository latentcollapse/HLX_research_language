//! HLXL (ASCII) Parser and Emitter

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, multispace0, multispace1, digit1, one_of},
    combinator::{opt, map, map_res, value},
    multi::{many0, separated_list0},
    sequence::{tuple, terminated},
    error::VerboseError,
};

use crate::ast::*;
use crate::parser::Parser;
use crate::emitter::Emitter;
use hlx_core::{Result, HlxError};

/// HLXL Parser
pub struct HlxlParser;

impl HlxlParser {
    pub fn new() -> Self { Self }
}

impl Default for HlxlParser {
    fn default() -> Self { Self::new() }
}

type ParseResult<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

fn ws(input: &str) -> ParseResult<()> { value((), multispace0)(input) }
fn ws1(input: &str) -> ParseResult<()> { value((), multispace1)(input) }

fn is_ident_start(c: char) -> bool { c.is_alphabetic() || c == '_' }
fn is_ident_cont(c: char) -> bool { c.is_alphanumeric() || c == '_' }

fn ident(input: &str) -> ParseResult<String> {
    let (input, first) = take_while1(is_ident_start)(input)?;
    let (input, rest) = take_while(is_ident_cont)(input)?;
    Ok((input, format!("{}{}", first, rest)))
}

fn keyword<'a>(kw: &'static str) -> impl FnMut(&'a str) -> ParseResult<'a, &'a str> {
    move |input: &'a str| {
        let (input, matched) = tag(kw)(input)?;
        if input.chars().next().map_or(true, |c| !is_ident_cont(c)) {
            Ok((input, matched))
        } else {
            Err(nom::Err::Error(VerboseError { errors: vec![] }))
        }
    }
}

fn literal(input: &str) -> ParseResult<Literal> {
    alt((
        value(Literal::Null, keyword("null")),
        value(Literal::Bool(true), keyword("true")),
        value(Literal::Bool(false), keyword("false")),
        map(tuple((opt(char('-')), digit1, char('.'), digit1)), |(s, w, _, f)| {
            let n = format!("{}{}.{}", s.unwrap_or(' '), w, f);
            Literal::Float(n.trim().parse().unwrap())
        }),
        map(tuple((opt(char('-')), digit1)), |(s, d)| {
            let n = format!("{}{}", s.unwrap_or(' '), d);
            Literal::Int(n.trim().parse().unwrap())
        }),
        map(tuple((char::<&str, VerboseError<&str>>('"'), take_while(|c| c != '"'), char('"'))), |(_, s, _)| Literal::String(s.to_string())),
    ))(input)
}

fn expr(input: &str) -> ParseResult<Expr> {
    alt((
        map(literal, Expr::Literal),
        map(ident, Expr::Ident),
    ))(input)
}

fn spanned_expr(input: &str) -> ParseResult<Spanned<Expr>> {
    let (input, e) = expr(input)?;
    Ok((input, Spanned::dummy(e)))
}

fn statement(input: &str) -> ParseResult<Statement> {
    alt((
        map(tuple((keyword("let"), ws1, ident, ws, char('='), ws, spanned_expr)), |(_, _, n, _, _, _, v)| Statement::Let { name: n, value: v }),
        map(tuple((keyword("return"), ws1, spanned_expr)), |(_, _, v)| Statement::Return { value: v }),
        // If
        map(tuple((
            keyword("if"), ws, spanned_expr, ws, char('{'), ws, many0(terminated(spanned_stmt, ws)), char('}'),
            opt(tuple((ws, keyword("else"), ws, char('{'), ws, many0(terminated(spanned_stmt, ws)), char('}'))))
        )), |(_, _, cond, _, _, _, then_body, _, els)| {
            Statement::If {
                condition: cond,
                then_branch: then_body,
                else_branch: els.map(|(_, _, _, _, _, body, _)| body),
            }
        }),
        // While/Loop
        map(tuple((
            keyword("while"), ws, char('('), ws, spanned_expr, opt(tuple((ws, char(','), ws, digit1))), ws, char(')'), ws,
            char('{'), ws, many0(terminated(spanned_stmt, ws)), char('}')
        )), |(_, _, _, _, cond, max_iter_opt, _, _, _, _, _, body, _)| {
            let max_iter = max_iter_opt
                .map(|(_, _, _, d)| d.parse().unwrap())
                .unwrap_or(1000); // Default for legacy HLXL
            Statement::While {
                condition: cond,
                body,
                max_iter,
            }
        }),
    ))(input)
}

fn spanned_stmt(input: &str) -> ParseResult<Spanned<Statement>> {
    let (input, s) = statement(input)?;
    Ok((input, Spanned::dummy(s)))
}

fn block(input: &str) -> ParseResult<Block> {
    let (input, _) = keyword("block")(input)?;
    let (input, _) = ws1(input)?;
    let (input, name) = ident(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('(')(input)?;
    let (input, _) = char(')')(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('{')(input)?;
    let (input, _) = ws(input)?;
    let (input, body) = many0(terminated(spanned_stmt, ws))(input)?;
    let (input, _) = char('}')(input)?;
    Ok((input, Block { name, params: vec![], body }))
}

fn parse_program(input: &str) -> ParseResult<Program> {
    let (input, _) = ws(input)?;
    let (input, _) = keyword("program")(input)?;
    let (input, _) = ws1(input)?;
    let (input, name) = ident(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = char('{')(input)?;
    let (input, _) = ws(input)?;
    let (input, blocks) = many0(terminated(block, ws))(input)?;
    let (input, _) = char('}')(input)?;
    Ok((input, Program { name, blocks }))
}

impl Parser for HlxlParser {
    fn parse(&self, source: &str) -> Result<Program> {
        match parse_program(source) {
            Ok((_, p)) => Ok(p),
            Err(e) => Err(HlxError::parse(format!("{:?}", e))),
        }
    }
    fn name(&self) -> &'static str { "HLXL" }
}

/// HLXL Emitter
pub struct HlxlEmitter;
impl HlxlEmitter {
    pub fn new() -> Self { Self }
}
impl Default for HlxlEmitter {
    fn default() -> Self { Self::new() }
}

impl Emitter for HlxlEmitter {
    fn emit(&self, program: &Program) -> Result<String> {
        let mut out = String::new();
        out.push_str("program ");
        out.push_str(&program.name);
        out.push_str(" {\n");
        for b in &program.blocks {
            out.push_str("  block ");
            out.push_str(&b.name);
            out.push_str("() {\n");
            for s in &b.body {
                match &s.node {
                    Statement::Let { name, value } => {
                        out.push_str("    let ");
                        out.push_str(name);
                        out.push_str(" = ");
                        out.push_str(&format!("{:?}", value.node)); // Simplified
                        out.push('\n');
                    }
                    Statement::Return { value } => {
                        out.push_str("    return ");
                        out.push_str(&format!("{:?}", value.node));
                        out.push('\n');
                    }
                    _ => {}
                }
            }
            out.push_str("  }\n");
        }
        out.push_str("}\n");
        Ok(out)
    }
    fn name(&self) -> &'static str { "HLXL" }
}