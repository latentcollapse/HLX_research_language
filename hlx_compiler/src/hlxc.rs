//! HLX-C (Compute) Parser and Emitter
//!
//! Turing-complete front-end for Helix logic.

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, multispace0, multispace1, digit1},
    combinator::{opt, map, value, recognize},
    multi::{many0, separated_list0},
    sequence::{tuple, terminated, delimited, preceded},
    error::VerboseError,
};

use crate::ast::*;
use crate::parser::Parser;
use hlx_core::{Result, HlxError};

/// HLX-C Parser
pub struct HlxcParser;

impl HlxcParser {
    pub fn new() -> Self { Self }
}

impl Default for HlxcParser {
    fn default() -> Self { Self::new() }
}

type ParseResult<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

fn ws(input: &str) -> ParseResult<()> {
    let (input, _) = many0(alt((
        value((), multispace1),
        value((), tuple((tag("//"), take_while(|c| c != '\n'), opt(char('\n'))))),
    )))(input)?;
    Ok((input, ()))
}

fn ws1(input: &str) -> ParseResult<()> {
    ws(input)
}

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
    ))(input)
}

// === Expression Parsing with Precedence ===

fn primary_expr(input: &str) -> ParseResult<Expr> {
    alt((
        map(literal, Expr::Literal),
        map(ident, Expr::Ident),
        delimited(terminated(char('('), ws), expr, preceded(ws, char(')'))),
    ))(input)
}

fn op_expr(input: &str) -> ParseResult<Expr> {
    let (input, lhs) = primary_expr(input)?;
    let (input, ops) = many0(tuple((
        preceded(ws, alt((
            value(BinOp::Add, tag("+")),
            value(BinOp::Sub, tag("-")),
            value(BinOp::Mul, tag("*")),
            value(BinOp::Div, tag("/")),
            value(BinOp::Eq, tag("==")),
            value(BinOp::Ne, tag("!=")),
            value(BinOp::Le, tag("<=")),
            value(BinOp::Ge, tag(">=")),
            value(BinOp::Lt, tag("<")),
            value(BinOp::Gt, tag(">")),
        ))),
        preceded(ws, primary_expr)
    )))(input)?;

    // Fold the operations (ignoring precedence for this simple bootstrap)
    let res = ops.into_iter().fold(lhs, |acc, (op, rhs)| {
        Expr::BinOp {
            op,
            lhs: Box::new(Spanned::dummy(acc)),
            rhs: Box::new(Spanned::dummy(rhs)),
        }
    });

    Ok((input, res))
}

fn expr(input: &str) -> ParseResult<Expr> {
    op_expr(input)
}

fn spanned_expr(input: &str) -> ParseResult<Spanned<Expr>> {
    let (input, e) = expr(input)?;
    Ok((input, Spanned::dummy(e)))
}

// === Statements ===

fn statement(input: &str) -> ParseResult<Statement> {
    preceded(ws, alt((
        // Let: let x = expr;
        map(tuple((keyword("let"), ws1, ident, ws, char('='), ws, spanned_expr, ws, char(';'))), 
            |(_, _, n, _, _, _, v, _, _)| Statement::Let { name: n, value: v }),
        
        // Return: return expr;
        map(tuple((keyword("return"), ws1, spanned_expr, ws, char(';'))), 
            |(_, _, v, _, _)| Statement::Return { value: v }),
            
        // If: if expr { ... } else { ... }
        map(tuple((
            keyword("if"), ws, spanned_expr, ws, char('{'), ws, many0(terminated(spanned_stmt, ws)), preceded(ws, char('}')),
            opt(preceded(ws, tuple((keyword("else"), ws, char('{'), ws, many0(terminated(spanned_stmt, ws)), preceded(ws, char('}'))))))
        )), |(_, _, cond, _, _, _, then_body, _, els)| {
            Statement::If {
                condition: cond,
                then_branch: then_body,
                else_branch: els.map(|(_, _, _, _, body, _)| body),
            }
        }),
        
        // Loop: loop (cond, max_iter) { ... }
        map(tuple((
            keyword("loop"), ws, char('('), ws, spanned_expr, preceded(ws, char(',')), ws, digit1, preceded(ws, char(')')), ws,
            char('{'), ws, many0(terminated(spanned_stmt, ws)), preceded(ws, char('}'))
        )), |(_, _, _, _, cond, _, _, max_iter, _, _, _, _, body, _)| {
            Statement::While {
                condition: cond,
                body,
                max_iter: max_iter.parse().unwrap(),
            }
        }),
        
        // Expr statement: expr;
        map(terminated(spanned_expr, preceded(ws, char(';'))), Statement::Expr),
    )))(input)
}

fn spanned_stmt(input: &str) -> ParseResult<Spanned<Statement>> {
    let (input, s) = statement(input)?;
    Ok((input, Spanned::dummy(s)))
}

fn function_def(input: &str) -> ParseResult<Block> {
    let (input, _) = preceded(ws, keyword("fn"))(input)?;
    let (input, _) = ws1(input)?;
    let (input, name) = ident(input)?;
    let (input, _) = preceded(ws, char('('))(input)?;
    let (input, _) = separated_list0(delimited(ws, char(','), ws), ident)(input)?;
    let (input, _) = preceded(ws, char(')'))(input)?;
    let (input, _) = preceded(ws, tag("->"))(input)?;
    let (input, _) = preceded(ws, ident)(input)?; // Return type
    let (input, _) = preceded(ws, char('{'))(input)?;
    let (input, body) = many0(terminated(spanned_stmt, ws))(input)?;
    let (input, _) = preceded(ws, char('}'))(input)?;
    Ok((input, Block { name, params: vec![], body }))
}

fn parse_program(input: &str) -> ParseResult<Program> {
    use nom::multi::many1;
    let (input, functions) = preceded(ws, many1(terminated(function_def, ws)))(input)?;
    Ok((input, Program { name: "hlxc_module".to_string(), blocks: functions }))
}

impl Parser for HlxcParser {
    fn parse(&self, source: &str) -> Result<Program> {
        match parse_program(source) {
            Ok((_, p)) => Ok(p),
            Err(e) => Err(HlxError::parse(format!("{:?}", e))),
        }
    }
    fn name(&self) -> &'static str { "HLX-C" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_fn() {
        let source = r#"
            fn test() -> i32 {
                let x = 5;
                return x;
            }
        "#;
        let parser = HlxcParser::new();
        let program = parser.parse(source).unwrap();
        assert_eq!(program.blocks.len(), 1);
        assert_eq!(program.blocks[0].name, "test");
    }

    #[test]
    fn test_parse_loop_if() {
        let source = r#"
            fn main() -> i32 {
                let x = 0;
                loop (x < 5, 10) {
                    let x = 1;
                }
                if x {
                    return 100;
                } else {
                    return 200;
                }
            }
        "#;
        let parser = HlxcParser::new();
        let program = parser.parse(source).unwrap();
        assert_eq!(program.blocks[0].body.len(), 3);
    }
}