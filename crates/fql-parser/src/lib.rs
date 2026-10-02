use logos::{Logos, Span};
use fql_ast::{Query, SelectQuery, InsertQuery, Expr, BinaryOperator};
use std::path::PathBuf;
use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ParseError {
    #[error("Lexer error")]
    #[diagnostic(code(fql::lexer_error))]
    LexerError {
        #[source_code]
        src: String,
        #[label("unrecognized token")]
        span: Span,
    },
    #[error("Unexpected token")]
    #[diagnostic(code(fql::unexpected_token))]
    UnexpectedToken {
        #[source_code]
        src: String,
        #[label("this token")]
        span: Span,
    },
    #[error("Expected {expected}")]
    #[diagnostic(code(fql::expected))]
    Expected {
        expected: String,
        #[source_code]
        src: String,
        #[label("here")]
        span: Span,
    },
    #[error("Unexpected end of input")]
    #[diagnostic(code(fql::unexpected_eof))]
    UnexpectedEof {
        #[source_code]
        src: String,
    },
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token<'a> {
    #[regex("(?i)select")]
    Select,
    #[regex("(?i)recursive")]
    Recursive,
    #[regex("(?i)from")]
    From,
    #[regex("(?i)where")]
    Where,
    #[regex("(?i)insert")]
    Insert,
    #[regex("(?i)into")]
    Into,
    
    #[token("*")]
    Star,
    #[token(",")]
    Comma,
    #[token("=")]
    Eq,
    #[token(">")]
    Gt,
    #[token("<")]
    Lt,
    #[token(">=")]
    GtEq,
    #[token("<=")]
    LtEq,
    #[token("!=")]
    NotEq,

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#, |lex| lex.slice())]
    String(&'a str),

    #[regex(r"[a-zA-Z0-9_/\.~-]+", |lex| lex.slice())]
    IdentOrPath(&'a str),
}

pub struct Parser<'a> {
    lexer: logos::Lexer<'a, Token<'a>>,
    current: Option<Result<Token<'a>, ()>>,
    span: Span,
    src: &'a str,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut lexer = Token::lexer(src);
        let current = lexer.next();
        let span = lexer.span();
        Self {
            lexer,
            current,
            span,
            src,
        }
    }

    fn advance(&mut self) {
        self.current = self.lexer.next();
        self.span = self.lexer.span();
    }

    fn expect(&mut self, expected: Token<'a>, expected_str: &str) -> Result<(), ParseError> {
        match &self.current {
            Some(Ok(tok)) if tok == &expected => {
                self.advance();
                Ok(())
            }
            Some(Ok(_)) => Err(ParseError::Expected {
                expected: expected_str.to_string(),
                src: self.src.to_string(),
                span: self.span.clone(),
            }),
            Some(Err(_)) => Err(ParseError::LexerError {
                src: self.src.to_string(),
                span: self.span.clone(),
            }),
            None => Err(ParseError::UnexpectedEof {
                src: self.src.to_string(),
            }),
        }
    }

    fn parse_ident_or_path(&mut self) -> Result<String, ParseError> {
        match &self.current {
            Some(Ok(Token::IdentOrPath(s))) => {
                let res = s.to_string();
                self.advance();
                Ok(res)
            }
            Some(Ok(_)) => Err(ParseError::Expected {
                expected: "identifier or path".to_string(),
                src: self.src.to_string(),
                span: self.span.clone(),
            }),
            Some(Err(_)) => Err(ParseError::LexerError {
                src: self.src.to_string(),
                span: self.span.clone(),
            }),
            None => Err(ParseError::UnexpectedEof {
                src: self.src.to_string(),
            }),
        }
    }

    pub fn parse(&mut self) -> Result<Query, ParseError> {
        match &self.current {
            Some(Ok(Token::Select)) => self.parse_select().map(Query::Select),
            Some(Ok(Token::Insert)) => self.parse_insert().map(Query::Insert),
            Some(Ok(_)) => Err(ParseError::UnexpectedToken {
                src: self.src.to_string(),
                span: self.span.clone(),
            }),
            Some(Err(_)) => Err(ParseError::LexerError {
                src: self.src.to_string(),
                span: self.span.clone(),
            }),
            None => Err(ParseError::UnexpectedEof {
                src: self.src.to_string(),
            }),
        }
    }

    fn parse_select(&mut self) -> Result<SelectQuery, ParseError> {
        self.expect(Token::Select, "select")?;
        
        let recursive = if let Some(Ok(Token::Recursive)) = self.current {
            self.advance();
            true
        } else {
            false
        };

        let mut fields = Vec::new();
        if let Some(Ok(Token::Star)) = self.current {
            fields.push("*".to_string());
            self.advance();
        } else {
            loop {
                fields.push(self.parse_ident_or_path()?);
                if let Some(Ok(Token::Comma)) = self.current {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(Token::From, "from")?;
        let from_str = self.parse_ident_or_path()?;
        let mut from = PathBuf::new();
        if from_str.starts_with("~/") || from_str == "~" {
            if let Some(home) = dirs::home_dir() {
                from.push(home);
                if from_str.len() > 1 {
                    from.push(&from_str[2..]); // Append the rest of the path after ~/
                }
            } else {
                from.push(from_str);
            }
        } else {
            from.push(from_str);
        }

        let where_clause = if let Some(Ok(Token::Where)) = self.current {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(SelectQuery {
            fields,
            recursive,
            from,
            where_clause,
        })
    }

    fn parse_insert(&mut self) -> Result<InsertQuery, ParseError> {
        self.expect(Token::Insert, "insert")?;
        
        let content = match &self.current {
            Some(Ok(Token::String(s))) => {
                // Remove quotes
                let res = s[1..s.len()-1].to_string();
                self.advance();
                res
            }
            _ => return Err(ParseError::Expected {
                expected: "string literal".to_string(),
                src: self.src.to_string(),
                span: self.span.clone(),
            })
        };

        self.expect(Token::Into, "into")?;
        
        let into_str = self.parse_ident_or_path()?;
        let mut into = PathBuf::new();
        if into_str.starts_with("~/") || into_str == "~" {
            if let Some(home) = dirs::home_dir() {
                into.push(home);
                if into_str.len() > 1 {
                    into.push(&into_str[2..]);
                }
            } else {
                into.push(into_str);
            }
        } else {
            into.push(into_str);
        }

        Ok(InsertQuery {
            content,
            into,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_ident_or_path()?;
        let left_expr = Expr::Identifier(left);

        let op = match &self.current {
            Some(Ok(Token::Eq)) => BinaryOperator::Eq,
            Some(Ok(Token::Gt)) => BinaryOperator::Gt,
            Some(Ok(Token::Lt)) => BinaryOperator::Lt,
            Some(Ok(Token::GtEq)) => BinaryOperator::GtEq,
            Some(Ok(Token::LtEq)) => BinaryOperator::LtEq,
            Some(Ok(Token::NotEq)) => BinaryOperator::NotEq,
            _ => return Err(ParseError::Expected {
                expected: "binary operator (=, >, <, >=, <=, !=)".to_string(),
                src: self.src.to_string(),
                span: self.span.clone(),
            })
        };
        self.advance(); // consume operator

        let right_expr = self.parse_rhs()?;

        Ok(Expr::BinaryOp {
            left: Box::new(left_expr),
            op,
            right: Box::new(right_expr),
        })
    }

    fn parse_rhs(&mut self) -> Result<Expr, ParseError> {
        // rhs can be a single value, or a list of values separated by commas
        let mut vals = Vec::new();
        loop {
            vals.push(self.parse_single_val()?);
            if let Some(Ok(Token::Comma)) = self.current {
                self.advance();
            } else {
                break;
            }
        }
        
        if vals.len() == 1 {
            Ok(vals.pop().unwrap())
        } else {
            Ok(Expr::List(vals))
        }
    }

    fn parse_single_val(&mut self) -> Result<Expr, ParseError> {
        match &self.current {
            Some(Ok(Token::IdentOrPath(s))) => {
                let res = s.to_string();
                self.advance();
                // Check if it's duration or int
                if res.ends_with('s') {
                    if let Ok(num) = res[..res.len()-1].parse::<u64>() {
                        return Ok(Expr::LiteralDuration(num));
                    }
                }
                if let Ok(num) = res.parse::<i64>() {
                    return Ok(Expr::LiteralInt(num));
                }
                Ok(Expr::LiteralString(res))
            }
            Some(Ok(Token::String(s))) => {
                let res = s[1..s.len()-1].to_string();
                self.advance();
                Ok(Expr::LiteralString(res))
            }
            _ => Err(ParseError::Expected {
                expected: "value".to_string(),
                src: self.src.to_string(),
                span: self.span.clone(),
            })
        }
    }
}

pub fn parse(src: &str) -> Result<Query, ParseError> {
    let mut parser = Parser::new(src);
    parser.parse()
}
