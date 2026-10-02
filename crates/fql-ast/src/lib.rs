use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Select(SelectQuery),
    Insert(InsertQuery),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectQuery {
    pub fields: Vec<String>,
    pub recursive: bool,
    pub from: PathBuf,
    pub where_clause: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InsertQuery {
    pub content: String,
    pub into: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },
    Identifier(String),
    LiteralString(String),
    LiteralInt(i64),
    LiteralDuration(u64), // seconds
    List(Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Eq,
    Gt,
    Lt,
    GtEq,
    LtEq,
    NotEq,
}
