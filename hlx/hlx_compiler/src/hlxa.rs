//! HLX-A (ASCII) Parser and Emitter

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, multispace1, digit1, one_of},
    combinator::{opt, map, value},
    multi::{many0, separated_list0},
    sequence::{tuple, delimited, preceded, terminated},
    error::VerboseError,
};
use tracing::{instrument, debug, trace};

use crate::ast::*;
use crate::parser::Parser;
use crate::emitter::Emitter;
use hlx_core::{Result, HlxError};

type ParseResult<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

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

fn array_literal(input: &str) -> ParseResult<'_, Expr> {
    map(delimited(
        preceded(ws, char('[')),
        separated_list0(preceded(ws, char(',')), expr),
        preceded(ws, char(']'))
    ), |elems| Expr::Array(elems.into_iter().map(Spanned::dummy).collect()))(input)
}

fn object_literal(input: &str) -> ParseResult<'_, Expr> {
    map(delimited(
        preceded(ws, char('{')),
        separated_list0(preceded(ws, char(',')), 
            tuple((
                preceded(ws, alt((
                    map(delimited(char('"'), take_while(|c| c != '"'), char('"')), |s: &str| s.to_string()),
                    map(delimited(char('\''), take_while(|c| c != '\''), char('\'')), |s: &str| s.to_string()),
                    ident
                ))),
                preceded(ws, char(':')),
                expr
            ))
        ),
        preceded(ws, char('}'))
    ), |items| Expr::Object(items.into_iter().map(|(k, _, v)| (k, Spanned::dummy(v))).collect()))(input)
}

fn call_expr(input: &str) -> ParseResult<'_, Expr> {
    let (input, name) = preceded(ws, ident)(input)?;
    let (input, args) = delimited(
        preceded(ws, char('(')),
        separated_list0(preceded(ws, char(',')), expr),
        preceded(ws, char(')'))
    )(input)?;
    Ok((input, Expr::Call {
        func: Box::new(Spanned::dummy(Expr::Ident(name))), 
        args: args.into_iter().map(Spanned::dummy).collect() 
    }))
}

enum Postfix {
    Index(Expr),
    Field(String),
}

#[instrument(skip(input), fields(preview = %&input[..input.len().min(30)].replace('\n', " ")))]
fn atom_expr(input: &str) -> ParseResult<'_, Expr> {
    debug!("Parsing atom");
    let (input, atom) = alt((
        map(preceded(ws, literal), Expr::Literal),
        array_literal,
        object_literal,
        call_expr,
        map(preceded(ws, ident), Expr::Ident),
        delimited(preceded(ws, char('(')), expr, preceded(ws, char(')'))),
    ))(input)?;
    
    let (input, postfixes) = many0(alt((
        map(delimited(
            preceded(ws, char('[')),
            expr,
            preceded(ws, char(']'))
        ), Postfix::Index),
        map(preceded(
            preceded(ws, char('.')),
            ident
        ), Postfix::Field)
    )))(input)?;
    
    Ok((input, postfixes.into_iter().fold(atom, |acc, op| match op {
        Postfix::Index(idx) => Expr::Index {
            object: Box::new(Spanned::dummy(acc)),
            index: Box::new(Spanned::dummy(idx))
        },
        Postfix::Field(field) => Expr::Field {
            object: Box::new(Spanned::dummy(acc)),
            field
        }
    })))
}

fn unary_expr(input: &str) -> ParseResult<'_, Expr> {
    alt((
        map(preceded(preceded(ws, char('-')), atom_expr), |e| Expr::UnaryOp { op: UnaryOp::Neg, operand: Box::new(Spanned::dummy(e)) }),
        map(preceded(preceded(ws, alt((tag("not"), tag("!")))), atom_expr), |e| Expr::UnaryOp { op: UnaryOp::Not, operand: Box::new(Spanned::dummy(e)) }),
        atom_expr
    ))(input)
}

fn bin_op(input: &str) -> ParseResult<'_, BinOp> {
    preceded(ws, alt((
        value(BinOp::Add, char('+')),
        value(BinOp::Sub, char('-')),
        value(BinOp::Mul, char('*')),
        value(BinOp::Div, char('/')),
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

#[instrument(skip(input), fields(preview = %&input[..input.len().min(30)].replace('\n', " ")))]
fn expr(input: &str) -> ParseResult<'_, Expr> {
    debug!("Parsing expr");
    let (input, lhs) = unary_expr(input)?;
    let (input, op_opt) = opt(bin_op)(input)?;
    
    if let Some(op) = op_opt {
        let (input, rhs) = expr(input)?;
        // Enforce no operator precedence: if RHS is a BinOp, it MUST be parenthesized
        // (Our recursive call to expr will handle nested BinOps, but we want to 
        // discourage this in the future or enforce it via the parser's structure)
        Ok((input, Expr::BinOp { 
            op, 
            lhs: Box::new(Spanned::dummy(lhs)), 
            rhs: Box::new(Spanned::dummy(rhs)) 
        }))
    } else {
        Ok((input, lhs))
    }
}

#[instrument(skip(input), fields(preview = %&input[..input.len().min(50)].replace('\n', " ")))]
fn statement(input: &str) -> ParseResult<'_, Statement> {
    debug!("Parsing statement");
    alt((
        map(tuple((preceded(ws, tag("let")), preceded(ws, ident), preceded(ws, char('=')), expr, preceded(ws, char(';')))), 
            |(_, n, _, v, _)| Statement::Let { name: n, value: Spanned::dummy(v) }),
        
        map(tuple((preceded(ws, tag("return")), expr, preceded(ws, char(';')))), 
            |(_, v, _)| Statement::Return { value: Spanned::dummy(v) }),
            
        map(tuple((
            preceded(ws, tag("if")), 
            alt((
                delimited(preceded(ws, char('(')), expr, preceded(ws, char(')'))),
                expr
            )),
            preceded(ws, char('{')), many0(map(preceded(ws, statement), Spanned::dummy)), preceded(ws, char('}')),
            opt(tuple((preceded(ws, tag("else")), preceded(ws, char('{')), many0(map(preceded(ws, statement), Spanned::dummy)), preceded(ws, char('}')))))
        )), |(_, cond, _, then_body, _, els)| {
            Statement::If { condition: Spanned::dummy(cond), then_branch: then_body, else_branch: els.map(|(_, _, v, _)| v) }
        }),
        
                map(tuple((
                    preceded(ws, tag("loop")), preceded(ws, char('(')), expr, preceded(ws, char(',')),
                    alt((
                        map(preceded(ws, digit1), |s: &str| s.parse::<u32>().unwrap()),
                        value(1000000, preceded(ws, tag("DEFAULT_MAX_ITER()")))
                    )),
                    preceded(ws, char(')')),
                    preceded(ws, char('{')), many0(map(preceded(ws, statement), Spanned::dummy)), preceded(ws, char('}'))
                )), |(_, _, cond, _, max_iter, _, _, body, _)| {
                    Statement::While { condition: Spanned::dummy(cond), body, max_iter }
                }),
        
                map(tuple((preceded(ws, tag("break")), preceded(ws, char(';')))), |(_, _)| Statement::Break),
                map(tuple((preceded(ws, tag("continue")), preceded(ws, char(';')))), |(_, _)| Statement::Continue),

                map(tuple((preceded(ws, atom_expr), preceded(ws, char('=')), expr, preceded(ws, char(';')))),
                    |(lhs, _, v, _)| Statement::Assign { lhs: Spanned::dummy(lhs), value: Spanned::dummy(v) }),
                
                map(preceded(ws, terminated(expr, preceded(ws, char(';')))), |e| Statement::Expr(Spanned::dummy(e))),    ))(input)
}

#[instrument(skip(input), fields(preview = %&input[..input.len().min(50)].replace('\n', " ")))]
fn block(input: &str) -> ParseResult<'_, Block> {
    debug!("Parsing block");
    alt((
        map(tuple((
            preceded(ws, tag("fn")), preceded(ws, ident), preceded(ws, char('(')),
            separated_list0(preceded(ws, char(',')), preceded(ws, ident)),
            preceded(ws, char(')')),
            opt(preceded(preceded(ws, tag("->")), preceded(ws, ident))),
            preceded(ws, char('{')), many0(map(preceded(ws, statement), Spanned::dummy)), preceded(ws, char('}'))
        )), |(_, name, _, params, _, return_type, _, body, _)| {
            let items = body.into_iter().map(|s| Spanned::new(Item::Statement(s.node), s.span)).collect();
            Block { name, params, return_type, items }
        }),
        map(tuple((
            preceded(ws, tag("block")), preceded(ws, ident), preceded(ws, char('(')), preceded(ws, char(')')),
            preceded(ws, char('{')), many0(map(preceded(ws, statement), Spanned::dummy)), preceded(ws, char('}'))
        )), |(_, name, _, _, _, body, _)| {
            let items = body.into_iter().map(|s| Spanned::new(Item::Statement(s.node), s.span)).collect();
            Block { name, params: vec![], return_type: None, items }
        }),
    ))(input)
}

#[instrument(skip(input), fields(preview = %&input[..input.len().min(50)].replace('\n', " ")))]
fn parse_program(input: &str) -> ParseResult<'_, Program> {
    debug!("Starting program parse");
    let (input, _) = preceded(ws, tag("program"))(input)?;
    let (input, name) = preceded(ws, ident)(input)?;
    let (input, _) = preceded(ws, char('{'))(input)?;
    let (input, blocks) = many0(preceded(ws, block))(input)?;
    let (input, _) = preceded(ws, char('}'))(input)?;
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
            Err(e) => Err(HlxError::parse(format!("{:?}", e))),
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
            Statement::Let { name, value } => {
                format!("{}let {} = {};", ind, name, self.emit_expr(&value.node))
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
            let params = block.params.join(", ");
            let return_type = block.return_type.as_ref().map(|t| format!(" -> {}", t)).unwrap_or_default();
            output.push_str(&format!("    fn {}({}){} {{\n", block.name, params, return_type));

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