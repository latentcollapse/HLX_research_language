//! HLX-A (ASCII) Parser and Emitter

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, multispace1, digit1, one_of},
    combinator::{opt, map, value, cut},
    multi::{many0, separated_list0},
    sequence::{tuple, delimited, preceded},
    error::{VerboseError, context},
};
// Tracing imports removed - not currently used

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

/// Parse a keyword and return its span (for semantic highlighting)
fn keyword_with_span<'a>(original: &'a str, keyword: &'static str, input: &'a str) -> ParseResult<'a, Span> {
    let (after_ws, _) = ws(input)?;
    let start_input = after_ws;
    let (remaining, _) = tag(keyword)(after_ws)?;
    let span = compute_span(original, start_input, remaining);
    Ok((remaining, span))
}

/// Parse an identifier and return both the identifier and its span
fn ident_with_span<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, (String, Span)> {
    let (after_ws, _) = ws(input)?;
    let start_input = after_ws;
    let (remaining, id) = ident(after_ws)?;
    let span = compute_span(original, start_input, remaining);
    Ok((remaining, (id, span)))
}

/// Parse a type annotation and return both the type and its span
fn type_with_span<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, (Type, Span)> {
    let (after_ws, _) = ws(input)?;
    let start_input = after_ws;
    let (remaining, typ) = parse_type(after_ws)?;
    let span = compute_span(original, start_input, remaining);
    Ok((remaining, (typ, span)))
}

fn is_ident_start(c: char) -> bool { c.is_alphabetic() || c == '_' }
fn is_ident_cont(c: char) -> bool { c.is_alphanumeric() || c == '_' }

fn ident(input: &str) -> ParseResult<'_, String> {
    let (input, first) = take_while1(is_ident_start)(input)?;
    let (input, rest) = take_while(is_ident_cont)(input)?;
    let id = format!("{}{}", first, rest);
    let keywords = ["let", "return", "if", "else", "loop", "program", "block", "fn", "null", "true", "false", "and", "or", "not", "break", "continue", "module", "enum", "struct", "const"];
    if keywords.contains(&id.as_str()) {
        return Err(nom::Err::Error(VerboseError {
            errors: vec![(input, nom::error::VerboseErrorKind::Context("keyword"))]
        }));
    }
    Ok((input, id))
}

fn parse_type(input: &str) -> ParseResult<'_, Type> {
    // Try to parse [T] array syntax
    if let Ok((input, _)) = char::<_, VerboseError<&str>>('[')(input) {
        let (input, inner) = preceded(ws, parse_type)(input)?;
        let (input, _) = preceded(ws, char(']'))(input)?;
        return Ok((input, Type::Array(Box::new(inner))));
    }
    
    // Try to parse Array<T> syntax
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
        // Hexadecimal integer (0x...)
        map(preceded(tag("0x"), take_while1(|c: char| c.is_ascii_hexdigit())), |hex: &str| {
            Literal::Int(i64::from_str_radix(hex, 16).unwrap())
        }),
        // Float
        map(tuple((opt(char('-')), digit1, char('.'), digit1)), |(s, w, _, f)| {
            let n = format!("{}{}.{}", s.unwrap_or(' '), w, f);
            Literal::Float(n.trim().parse().unwrap())
        }),
        // Decimal integer
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
    Call(Vec<Spanned<Expr>>),
    Cast(Type),
}

fn atom_expr<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Expr> {
    let (input, atom) = alt((
        map(preceded(ws, literal), Expr::Literal),
        |i| array_literal(original, i),
        |i| object_literal(original, i),
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
        ), Postfix::Field),
        map(delimited(
            preceded(ws, char('(')),
            separated_list0(preceded(ws, char(',')), |i| spanned(original, |i2| expr(original, i2))(i)),
            preceded(ws, char(')'))
        ), Postfix::Call),
        map(preceded(
            preceded(ws, tag("as")),
            preceded(ws, parse_type)
        ), Postfix::Cast)
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
            },
            Postfix::Call(args) => {
                (remaining, Expr::Call {
                    func: Box::new(Spanned::new(acc, acc_span)),
                    args
                })
            },
            Postfix::Cast(target_type) => {
                (remaining, Expr::Cast {
                    expr: Box::new(Spanned::new(acc, acc_span)),
                    target_type
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
        // Order matters! Longer operators must come before shorter ones
        value(BinOp::Eq, tag("==")),
        value(BinOp::Ne, tag("!=")),
        value(BinOp::Shl, tag("<<")),
        value(BinOp::Shr, tag(">>")),
        value(BinOp::Le, tag("<=")),
        value(BinOp::Ge, tag(">=")),
        value(BinOp::And, tag("and")),
        value(BinOp::Or, tag("or")),
        value(BinOp::Add, char('+')),
        value(BinOp::Sub, char('-')),
        value(BinOp::Mul, char('*')),
        value(BinOp::Div, char('/')),
        value(BinOp::Mod, char('%')),
        value(BinOp::Lt, char('<')),
        value(BinOp::Gt, char('>')),
        value(BinOp::BitAnd, char('&')),
        value(BinOp::BitOr, char('|')),
        value(BinOp::BitXor, char('^')),
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
                let (i, let_kw_span) = keyword_with_span(original, "let", i)?;
                let (i, (n, n_span)) = cut(context("variable name", |i2| ident_with_span(original, i2)))(i)?;
                let (i, (t, t_span)) = match opt(preceded(preceded(ws, char(':')), |i2| type_with_span(original, i2)))(i)? {
                    (i, Some((typ, span))) => (i, (Some(typ), Some(span))),
                    (i, None) => (i, (None, None)),
                };
                let (i, _) = cut(context("'=' after variable name", preceded(ws, char('='))))(i)?;
                let (i, v) = cut(context("expression", |i2| spanned(original, |i3| expr(original, i3))(i2)))(i)?;
                let (i, _) = cut(context("';' after let statement", preceded(ws, char(';'))))(i)?;
                Ok((i, Statement::Let {
                    keyword_span: Some(let_kw_span),
                    name: n,
                    name_span: Some(n_span),
                    type_annotation: t,
                    type_span: t_span,
                    value: v
                }))
            }
        ),

        // return value;
        context("return statement",
            |i| {
                let (i, ret_kw_span) = keyword_with_span(original, "return", i)?;
                let (i, v) = cut(context("expression", |i2| spanned(original, |i3| expr(original, i3))(i2)))(i)?;
                let (i, _) = cut(context("';' after return", preceded(ws, char(';'))))(i)?;
                Ok((i, Statement::Return {
                    keyword_span: Some(ret_kw_span),
                    value: v
                }))
            }
        ),

        // if condition { ... } else { ... }
        context("if statement",
            |i| {
                let (i, if_kw_span) = keyword_with_span(original, "if", i)?;
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
                let (i, else_opt) = opt(|i2| {
                    let (i2, else_kw_span) = keyword_with_span(original, "else", i2)?;
                    let (i2, _) = preceded(ws, char('{'))(i2)?;
                    let (i2, body) = many0(|i3| spanned(original, |i4| statement(original, i4))(i3))(i2)?;
                    let (i2, _) = preceded(ws, char('}'))(i2)?;
                    Ok((i2, (body, else_kw_span)))
                })(i)?;
                let (els, else_kw_span) = match else_opt {
                    Some((body, span)) => (Some(body), Some(span)),
                    None => (None, None),
                };
                Ok((i, Statement::If {
                    if_keyword_span: Some(if_kw_span),
                    condition: cond,
                    then_branch: then_body,
                    else_keyword_span: else_kw_span,
                    else_branch: els
                }))
            }
        ),

        // loop(condition, max_iter) { ... }
        context("loop statement",
            |i| {
                let (i, loop_kw_span) = keyword_with_span(original, "loop", i)?;
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
                Ok((i, Statement::While {
                    loop_keyword_span: Some(loop_kw_span),
                    condition: cond,
                    body,
                    max_iter
                }))
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

        // barrier; or barrier("name");
        context("barrier statement",
            |i| {
                let (i, barrier_kw_span) = keyword_with_span(original, "barrier", i)?;
                // Parse optional barrier name
                let (i, name) = opt(preceded(
                    preceded(ws, char('(')),
                    |i2| {
                        let (i2, s) = preceded(ws, parse_string_literal)(i2)?;
                        let (i2, _) = cut(context("')' after barrier name", preceded(ws, char(')'))))(i2)?;
                        Ok((i2, s))
                    }
                ))(i)?;
                let (i, _) = cut(context("';' after barrier", preceded(ws, char(';'))))(i)?;
                Ok((i, Statement::Barrier {
                    name,
                    keyword_span: Some(barrier_kw_span),
                }))
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

        // asm statement: asm("template" : outputs : inputs : clobbers);
        context("asm statement",
            |i| {
                let (i, _) = preceded(ws, tag("asm"))(i)?;
                let (i, _) = cut(context("'(' after asm", preceded(ws, char('('))))(i)?;
                
                // Parse template string
                let (i, template) = cut(context("asm template string", preceded(ws, parse_string_literal)))(i)?;
                
                // Parse optional outputs, inputs, clobbers (all separated by ':')
                let (i, outputs) = opt(preceded(
                    preceded(ws, char(':')),
                    separated_list0(preceded(ws, char(',')), |i2| {
                        let (i2, constraint) = preceded(ws, parse_string_literal)(i2)?;
                        let (i2, _) = preceded(ws, char('('))(i2)?;
                        let (i2, var) = preceded(ws, ident)(i2)?;
                        let (i2, _) = preceded(ws, char(')'))(i2)?;
                        Ok((i2, (constraint, var)))
                    })
                ))(i)?;
                
                let (i, inputs) = opt(preceded(
                    preceded(ws, char(':')),
                    separated_list0(preceded(ws, char(',')), |i2| {
                        let (i2, constraint) = preceded(ws, parse_string_literal)(i2)?;
                        let (i2, _) = preceded(ws, char('('))(i2)?;
                        let (i2, expr_val) = spanned(original, |i3| expr(original, i3))(i2)?;
                        let (i2, _) = preceded(ws, char(')'))(i2)?;
                        Ok((i2, (constraint, expr_val)))
                    })
                ))(i)?;
                
                let (i, clobbers) = opt(preceded(
                    preceded(ws, char(':')),
                    separated_list0(preceded(ws, char(',')), preceded(ws, parse_string_literal))
                ))(i)?;
                
                let (i, _) = cut(context("')' after asm", preceded(ws, char(')'))))(i)?;
                let (i, _) = cut(context("';' after asm", preceded(ws, char(';'))))(i)?;
                
                Ok((i, Statement::Asm {
                    template,
                    outputs: outputs.unwrap_or_default(),
                    inputs: inputs.unwrap_or_default(),
                    clobbers: clobbers.unwrap_or_default(),
                }))
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

/// Parse attributes like #[no_mangle], #[entry], @substrate(cpu), @scale(size=1000)
fn parse_attributes(input: &str) -> ParseResult<'_, Vec<String>> {
    many0(alt((
        // HLX-Scale pragmas: @substrate(...) or @scale(...)
        |i| {
            let (i, _) = preceded(ws, char('@'))(i)?;
            let (i, pragma_name) = preceded(ws, ident)(i)?;
            let (i, _) = preceded(ws, char('('))(i)?;
            // Parse the content inside parentheses (everything until ')')
            let (i, content) = take_while(|c| c != ')')(i)?;
            let (i, _) = char(')')(i)?;
            // Return as "name(content)" format
            Ok((i, format!("{}({})", pragma_name, content)))
        },
        // Traditional attributes: #[name] or #[name(value)]
        |i| {
            let (i, _) = preceded(ws, char('#'))(i)?;
            let (i, _) = preceded(ws, char('['))(i)?;
            let (i, attr_name) = preceded(ws, ident)(i)?;
            // Check for optional parameters: (...)
            let (i, params_opt) = opt(|i2| {
                let (i2, _) = preceded(ws, char('('))(i2)?;
                let (i2, content) = take_while(|c| c != ')')(i2)?;
                let (i2, _) = char(')')(i2)?;
                Ok((i2, content))
            })(i)?;
            let (i, _) = preceded(ws, char(']'))(i)?;
            let attr_str = if let Some(params) = params_opt {
                format!("{}({})", attr_name, params)
            } else {
                attr_name
            };
            Ok((i, attr_str))
        }
    )))(input)
}

fn block<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Block> {
    alt((
        // fn name(params) -> return_type { body }
        context("function definition",
            |i| {
                let (i, attributes) = parse_attributes(i)?;
                let (i, fn_kw_span) = keyword_with_span(original, "fn", i)?;
                let (i, (name, name_span)) = cut(context("function name", |i2| ident_with_span(original, i2)))(i)?;
                let (i, _) = cut(context("'(' for parameters", preceded(ws, char('('))))(i)?;
                let (i, params) = separated_list0(preceded(ws, char(',')),
                    |i2| {
                        let (i2, (param_name, param_span)) = ident_with_span(original, i2)?;
                        let (i2, typ_opt) = opt(preceded(preceded(ws, char(':')), |i3| type_with_span(original, i3)))(i2)?;
                        let typ_with_span = typ_opt.map(|(t, s)| (t, Some(s)));
                        Ok((i2, (param_name, Some(param_span), typ_with_span)))
                    }
                )(i)?;
                let (i, _) = cut(context("')' after parameters", preceded(ws, char(')'))))(i)?;
                let (i, (return_type, return_type_span)) = match opt(preceded(preceded(ws, tag("->")), |i2| type_with_span(original, i2)))(i)? {
                    (i, Some((t, s))) => (i, (Some(t), Some(s))),
                    (i, None) => (i, (None, None)),
                };
                let (i, _) = cut(context("'{' to start function body", preceded(ws, char('{'))))(i)?;
                let (i, body) = many0(|i2| spanned(original, |i3| statement(original, i3))(i2))(i)?;
                let (i, _) = cut(context("'}' to close function", preceded(ws, char('}'))))(i)?;
                let items = body.into_iter().map(|s| Spanned::new(Item::Statement(s.node), s.span)).collect();
                Ok((i, Block {
                    name,
                    attributes: attributes.clone(),
                    name_span: Some(name_span),
                    fn_keyword_span: Some(fn_kw_span),
                    params,
                    return_type,
                    return_type_span,
                    items
                }))
            }
        ),
        // block name() { body }
        context("block definition",
            |i| {
                let (i, attributes) = parse_attributes(i)?;
                let (i, _) = preceded(ws, tag("block"))(i)?;
                let (i, name) = cut(context("block name", preceded(ws, ident)))(i)?;
                let (i, _) = preceded(ws, char('('))(i)?;
                let (i, _) = preceded(ws, char(')'))(i)?;
                let (i, _) = cut(context("'{' to start block body", preceded(ws, char('{'))))(i)?;
                let (i, body) = many0(|i2| spanned(original, |i3| statement(original, i3))(i2))(i)?;
                let (i, _) = cut(context("'}' to close block", preceded(ws, char('}'))))(i)?;
                let items = body.into_iter().map(|s| Spanned::new(Item::Statement(s.node), s.span)).collect();
                Ok((i, Block {
                    name,
                    attributes: attributes.clone(),
                    name_span: None,
                    fn_keyword_span: None,
                    params: vec![],
                    return_type: None,
                    return_type_span: None,
                    items
                }))
            }
        ),
    ))(input)
}

fn parse_program(input: &str) -> ParseResult<'_, Program> {
    let original = input; // Save original for position tracking
    
    // Check if this is a standalone module by looking for the 'module' keyword
    let (after_ws, _) = ws(input)?;
    if let Ok((_, _)) = tag::<_, _, VerboseError<&str>>("module")(after_ws) {
        // This is a standalone module
        let (remaining, module) = parse_module(original, input)?;
        return Ok((remaining, Program {
            name: module.name.clone(),
            imports: vec![],
            modules: vec![module],
            blocks: vec![],
        }));
    }
    
    // Otherwise, parse as a full program
    let (input, _) = context("'program' keyword", preceded(ws, tag("program")))(input)?;
    let (input, name) = cut(context("program name", preceded(ws, ident)))(input)?;
    let (input, _) = cut(context("'{' to start program", preceded(ws, char('{'))))(input)?;
    let (input, modules) = many0(preceded(ws, |i| parse_module(original, i)))(input)?;
    let (input, blocks) = many0(preceded(ws, |i| block(original, i)))(input)?;
    let (input, _) = cut(context("'}' to close program", preceded(ws, char('}'))))(input)?;
    Ok((input, Program { name, imports: vec![], modules, blocks }))
}

fn parse_module<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Module> {
    let (input, _) = context("'module' keyword", preceded(ws, tag("module")))(input)?;
    let (input, name) = cut(context("module name", preceded(ws, ident)))(input)?;
    let (input, _) = cut(context("'{\' to start module", preceded(ws, char('{'))))(input)?;

    // Parse capabilities (optional)
    // Syntax: capability: cap1, cap2, cap3
    let (input, capabilities) = opt(
        preceded(
            tuple((context("'capability' keyword", preceded(ws, tag("capability"))), preceded(ws, char(':')))),
            cut(separated_list0(preceded(ws, char(',')), preceded(ws, ident)))
        )
    )(input)?;

    // Parse module items (constants, structs, enums, functions) in any order
    enum ModuleItem {
        Constant(Constant),
        Struct(StructDef),
        Enum(EnumDef),
        Block(Block),
    }

    let (input, items) = many0(preceded(ws, alt((
        map(|i| parse_constant(original, i), ModuleItem::Constant),
        map(|i| parse_struct_def(original, i), ModuleItem::Struct),
        map(|i| parse_enum_def(original, i), ModuleItem::Enum),
        map(|i| block(original, i), ModuleItem::Block),
    ))))(input)?;

    // Separate items by type
    let mut constants = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut blocks = Vec::new();

    for item in items {
        match item {
            ModuleItem::Constant(c) => constants.push(c),
            ModuleItem::Struct(s) => structs.push(s),
            ModuleItem::Enum(e) => enums.push(e),
            ModuleItem::Block(b) => blocks.push(b),
        }
    }

    let (input, _) = cut(context("'}' to close module", preceded(ws, char('}'))))(input)?;

    Ok((input, Module {
        name,
        capabilities: capabilities.unwrap_or_default(),
        imports: vec![],
        exports: vec![],
        constants,
        structs,
        enums,
        blocks,
    }))
}

fn parse_constant<'a>(original: &'a str, input: &'a str) -> ParseResult<'a, Constant> {
    let (input, _) = context("'const' keyword", preceded(ws, tag("const")))(input)?;
    let (input, name) = cut(context("constant name", preceded(ws, ident)))(input)?;
    let (input, _) = cut(context("':' after constant name", preceded(ws, char(':'))))(input)?;
    let (input, typ) = cut(context("constant type", preceded(ws, parse_type)))(input)?;
    let (input, _) = cut(context("'=' after constant type", preceded(ws, char('='))))(input)?;
    let (input, value) = cut(context("constant value", spanned(original, |i| expr(original, i))))(input)?;
    let (input, _) = cut(context("';' after constant value", preceded(ws, char(';'))))(input)?;
    Ok((input, Constant { name, typ, value }))
}

fn parse_struct_def<'a>(_original: &'a str, input: &'a str) -> ParseResult<'a, StructDef> {
    let (input, _) = context("'struct' keyword", preceded(ws, tag("struct")))(input)?;
    let (input, name) = cut(context("struct name", preceded(ws, ident)))(input)?;
    let (input, _) = cut(context("'{' to start struct fields", preceded(ws, char('{'))))(input)?;
    
    // Parse fields - separated by commas or just whitespace, with optional trailing comma
    let (input, fields) = many0(|i| {
        let (i, _) = ws(i)?;
        // Try to parse a field
        let (i, field_name) = ident(i)?;
        let (i, _) = preceded(ws, char(':'))(i)?;
        let (i, field_type) = preceded(ws, parse_type)(i)?;
        // Optional trailing comma
        let (i, _) = opt(preceded(ws, char(',')))(i)?;
        Ok((i, (field_name, field_type)))
    })(input)?;
    
    let (input, _) = cut(context("'}' to close struct fields", preceded(ws, char('}'))))(input)?;
    Ok((input, StructDef { name, fields }))
}

fn parse_enum_def<'a>(_original: &'a str, input: &'a str) -> ParseResult<'a, EnumDef> {
    let (input, _) = context("'enum' keyword", preceded(ws, tag("enum")))(input)?;
    let (input, name) = cut(context("enum name", preceded(ws, ident)))(input)?;
    let (input, _) = cut(context("'{' to start enum variants", preceded(ws, char('{'))))(input)?;
    
    // Parse variants - separated by commas or just whitespace, with optional trailing comma
    let (input, variants) = many0(|i| {
        let (i, _) = ws(i)?;
        // Try to parse a variant
        let (i, variant_name) = ident(i)?;
        // Optional trailing comma
        let (i, _) = opt(preceded(ws, char(',')))(i)?;
        Ok((i, variant_name))
    })(input)?;
    
    let (input, _) = cut(context("'}' to close enum variants", preceded(ws, char('}'))))(input)?;
    Ok((input, EnumDef { name, variants }))
}

pub struct HlxaParser;
impl HlxaParser {
    pub fn new() -> Self { Self }

    pub fn parse_diagnostics(&self, source: &str) -> std::result::Result<Program, Vec<(String, usize)>> {
        match parse_program(source) {
            Ok((_, p)) => Ok(p),
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                let mut diags = Vec::new();

                // Get all errors from the error chain (nom collects multiple errors)
                for (substring, kind) in &e.errors {
                    let offset = substring.as_ptr() as usize - source.as_ptr() as usize;
                    diags.push((format!("{:?}", kind), offset));
                }

                // If no errors were collected, add a generic one
                if diags.is_empty() {
                    diags.push(("Unknown syntax error".to_string(), 0));
                }

                // Try to find additional errors by parsing individual lines
                // This helps catch multiple independent syntax errors
                let additional_errors = self.find_additional_errors(source, &diags);
                diags.extend(additional_errors);

                // Deduplicate errors at the same location
                diags.sort_by_key(|(_, offset)| *offset);
                diags.dedup_by_key(|(_, offset)| *offset);

                Err(diags)
            },
            Err(nom::Err::Incomplete(_)) => Err(vec![("Incomplete input".to_string(), source.len())]),
        }
    }

    /// Attempt to find additional syntax errors by parsing the document in smaller chunks
    fn find_additional_errors(&self, source: &str, existing_errors: &[(String, usize)]) -> Vec<(String, usize)> {
        let mut additional_errors = Vec::new();

        // Get existing error positions to avoid duplicates
        let error_positions: std::collections::HashSet<_> =
            existing_errors.iter().map(|(_, pos)| *pos).collect();

        // Try to parse individual functions and statements
        // This can catch errors in later parts of the file
        let lines: Vec<&str> = source.lines().collect();
        let mut current_offset = 0;
        let mut brace_depth = 0;
        let mut in_function = false;
        let mut function_start = 0;

        for (line_idx, line) in lines.iter().enumerate() {
            let line_len = line.len() + 1; // +1 for newline

            // Track brace depth to find function boundaries
            for ch in line.chars() {
                match ch {
                    '{' => {
                        brace_depth += 1;
                        if !in_function && brace_depth == 1 {
                            in_function = true;
                            function_start = current_offset;
                        }
                    }
                    '}' => {
                        brace_depth -= 1;
                        if in_function && brace_depth == 0 {
                            // Calculate end position safely
                            let function_end = std::cmp::min(current_offset + line_len, source.len());

                            // Only try to parse if we have valid bounds
                            if function_start < function_end {
                                let function_text = &source[function_start..function_end];

                                // Attempt to parse as a statement or expression
                                // If it fails and we don't already have an error here, record it
                                if let Err(e) = self.try_parse_fragment(function_text) {
                                    if !error_positions.contains(&(function_start + e)) {
                                        additional_errors.push((
                                            "Syntax error in block".to_string(),
                                            function_start + e,
                                        ));
                                    }
                                }
                            }

                            in_function = false;
                        }
                    }
                    _ => {}
                }
            }

            current_offset += line_len;
        }

        // Limit to avoid overwhelming the user with too many errors
        additional_errors.truncate(10);
        additional_errors
    }

    /// Try to parse a fragment of code to detect errors
    fn try_parse_fragment(&self, fragment: &str) -> std::result::Result<(), usize> {
        // Try parsing as various constructs
        // If any succeed, the fragment is valid; if all fail, return the earliest error position

        // This is a simplified check - just verify it's not obviously broken
        // A full implementation would try to parse as different statement types

        let mut error_pos = 0;
        let mut found_error = false;

        // Check for unmatched braces/parens
        let mut brace_count = 0;
        let mut paren_count = 0;

        for (idx, ch) in fragment.chars().enumerate() {
            match ch {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count < 0 {
                        error_pos = idx;
                        found_error = true;
                        break;
                    }
                }
                '(' => paren_count += 1,
                ')' => {
                    paren_count -= 1;
                    if paren_count < 0 {
                        error_pos = idx;
                        found_error = true;
                        break;
                    }
                }
                _ => {}
            }
        }

        if found_error || brace_count != 0 || paren_count != 0 {
            Err(error_pos)
        } else {
            Ok(())
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
                    BinOp::BitAnd => "&",
                    BinOp::BitOr => "|",
                    BinOp::BitXor => "^",
                    BinOp::Shl => "<<",
                    BinOp::Shr => ">>",
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
            Statement::Let { name, type_annotation, value, .. } => {
                if let Some(typ) = type_annotation {
                    format!("{}let {}: {} = {};", ind, name, typ.to_string(), self.emit_expr(&value.node))
                } else {
                    format!("{}let {} = {};", ind, name, self.emit_expr(&value.node))
                }
            },
            Statement::Assign { lhs, value } => {
                format!("{}{} = {};", ind, self.emit_expr(&lhs.node), self.emit_expr(&value.node))
            },
            Statement::Return { value, .. } => {
                format!("{}return {};", ind, self.emit_expr(&value.node))
            },
            Statement::If { condition, then_branch, else_branch, .. } => {
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
            Statement::While { condition, body, max_iter, .. } => {
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
                .map(|(name, _name_span, typ_opt)| {
                    if let Some((t, _type_span)) = typ_opt {
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

        // With enhanced error recovery, we may get multiple errors
        // Just verify we got at least one error
        println!("Found {} error(s):", errors.len());
        for (msg, offset) in &errors {
            println!("  - {} at offset {}", msg, offset);
        }
    }

    #[test]
    fn test_multi_error_reporting() {
        let parser = HlxaParser::new();

        // Source with multiple syntax errors:
        // 1. Incomplete expression (1 +)
        // 2. Mismatched parens
        // 3. Unmatched braces
        let source = r#"
program test {
    fn foo() {
        let x = 1 + ;
        return x;
    }

    fn bar() {
        let y = (2 + 3;
        return y;

    fn baz() {
        return 1;
    }
}
"#;

        let result = parser.parse_diagnostics(source);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        println!("Found {} error(s):", errors.len());
        for (msg, offset) in &errors {
            println!("  - {} at offset {}", msg, offset);
        }

        // We should detect at least one error (the main parsing error)
        // Additional errors may be detected by the enhanced error recovery
        assert!(!errors.is_empty());

        // The enhanced parser may find multiple errors
        if errors.len() > 1 {
            println!("✓ Successfully detected multiple errors!");
        }
    }

    #[test]
    fn test_valid_program_no_errors() {
        let parser = HlxaParser::new();

        let source = r#"
program test {
    fn main() {
        let x = 1;
        let y = 2;
        return x + y;
    }
}
"#;

        let result = parser.parse_diagnostics(source);
        assert!(result.is_ok());

        let program = result.unwrap();
        assert_eq!(program.blocks.len(), 1);
    }
}