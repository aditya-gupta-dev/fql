use fql_ast::{BinaryOperator, Expr, InsertQuery, Query, SelectQuery};
use jwalk::WalkDir;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FileRecord {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub type_: String, // "f", "d", "s"
    pub modified_at: SystemTime,
    pub created_at: SystemTime,
}

#[derive(Debug)]
pub enum EngineError {
    IoError(std::io::Error),
    InvalidPath(PathBuf),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::IoError(e) => write!(f, "IO Error: {}", e),
            EngineError::InvalidPath(p) => write!(f, "Invalid path: {}", p.display()),
        }
    }
}
impl std::error::Error for EngineError {}

pub fn execute_query(query: Query) -> Result<Vec<FileRecord>, EngineError> {
    match query {
        Query::Select(sq) => execute_select(sq),
        Query::Insert(iq) => {
            execute_insert(iq)?;
            Ok(vec![])
        }
    }
}

fn execute_select(sq: SelectQuery) -> Result<Vec<FileRecord>, EngineError> {
    if !sq.from.exists() {
        return Err(EngineError::InvalidPath(sq.from));
    }

    let mut records = Vec::new();

    // jwalk is recursive. If sq.recursive is false, max_depth = 1.
    let depth = if sq.recursive { usize::MAX } else { 1 };

    for entry in WalkDir::new(&sq.from).max_depth(depth).skip_hidden(false) {
        if let Ok(entry) = entry {
            if entry.depth == 0 {
                continue;
            }

            let meta = if let Ok(m) = entry.metadata() {
                m
            } else {
                continue;
            };

            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let size = meta.len();

            let type_ = if meta.is_dir() {
                "d".to_string()
            } else if meta.is_symlink() {
                "s".to_string()
            } else {
                "f".to_string()
            };

            let modified_at = meta.modified().unwrap_or_else(|_| SystemTime::now());
            let created_at = meta.created().unwrap_or_else(|_| SystemTime::now());

            let record = FileRecord {
                name,
                path,
                size,
                type_,
                modified_at,
                created_at,
            };

            if let Some(ref expr) = sq.where_clause {
                if !evaluate_expr(expr, &record) {
                    continue;
                }
            }

            records.push(record);
        }
    }

    Ok(records)
}

fn evaluate_expr(expr: &Expr, record: &FileRecord) -> bool {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            if let Expr::Identifier(ref field) = **left {
                evaluate_condition(field, op, right, record)
            } else {
                false // only simple WHERE field = value is supported for now
            }
        }
        _ => false,
    }
}

fn evaluate_condition(field: &str, op: &BinaryOperator, right: &Expr, record: &FileRecord) -> bool {
    match field {
        "type" => {
            let matches = match right {
                Expr::LiteralString(s) => vec![s.clone()],
                Expr::List(list) => list
                    .iter()
                    .filter_map(|e| {
                        if let Expr::LiteralString(s) = e {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect(),
                _ => vec![],
            };
            if matches.is_empty() {
                return false;
            }
            match op {
                BinaryOperator::Eq => matches.contains(&record.type_),
                BinaryOperator::NotEq => !matches.contains(&record.type_),
                _ => false,
            }
        }
        "created_at" | "modified_at" => {
            let rec_time = if field == "created_at" {
                record.created_at
            } else {
                record.modified_at
            };
            let duration_secs = match right {
                Expr::LiteralDuration(s) => *s,
                _ => return false,
            };
            // e.g. created_at > 10s means created MORE than 10 seconds ago
            // So rec_time + 10s < now
            let now = SystemTime::now();
            if let Ok(age) = now.duration_since(rec_time) {
                let age_secs = age.as_secs();
                match op {
                    BinaryOperator::Gt => age_secs > duration_secs,
                    BinaryOperator::Lt => age_secs < duration_secs,
                    BinaryOperator::GtEq => age_secs >= duration_secs,
                    BinaryOperator::LtEq => age_secs <= duration_secs,
                    BinaryOperator::Eq => age_secs == duration_secs,
                    BinaryOperator::NotEq => age_secs != duration_secs,
                }
            } else {
                false // file from the future?
            }
        }
        "name" => {
            let val = match right {
                Expr::LiteralString(s) => s.clone(),
                _ => return false,
            };
            match op {
                BinaryOperator::Eq => record.name == val,
                BinaryOperator::NotEq => record.name != val,
                _ => false,
            }
        }
        _ => false,
    }
}

fn execute_insert(iq: InsertQuery) -> Result<(), EngineError> {
    if let Some(parent) = iq.into.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(EngineError::IoError)?;
        }
    }
    fs::write(&iq.into, &iq.content).map_err(EngineError::IoError)?;
    Ok(())
}
