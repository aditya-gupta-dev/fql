use clap::{Parser as ClapParser, ValueEnum};
use fql_parser::parse;
use fql_engine::execute_query;
use fql_server::start_server;
use tabled::builder::Builder;
use miette::{IntoDiagnostic, Result};
use std::io::{self, Read};
use tokio::time::timeout;
use std::time::Duration;
use chrono::{DateTime, Local};
use std::time::SystemTime;
use fql_ast::Query;

#[derive(ClapParser, Debug)]
#[command(author, version, about = "SQL for the filesystem", long_about = None)]
struct Args {
    query: Option<String>,
    #[arg(long)]
    timeout: Option<String>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    output: OutputFormat,
    #[arg(long)]
    stats: bool,
    #[arg(long)]
    server: bool,
    #[arg(long, default_value_t = 8080)]
    port: u16,
}

#[derive(ValueEnum, Clone, Debug)]
enum OutputFormat {
    Table,
    Json,
    Text,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.server {
        start_server(args.port).await;
        return Ok(());
    }

    let query_str = if let Some(q) = args.query {
        q
    } else {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).into_diagnostic()?;
        buffer
    };

    let start_time = std::time::Instant::now();

    let ast = parse(&query_str).map_err(|e| {
        let report: miette::Report = e.into();
        report.with_source_code(query_str.clone())
    })?;

    let fields = match &ast {
        Query::Select(sq) => sq.fields.clone(),
        _ => vec!["*".to_string()],
    };

    let duration = if let Some(t) = args.timeout {
        if t.ends_with("s") {
            let secs: u64 = t[..t.len()-1].parse().unwrap_or(2);
            Duration::from_secs(secs)
        } else {
            Duration::from_secs(2) 
        }
    } else {
        Duration::from_secs(u64::MAX) 
    };

    let result = timeout(duration, async {
        tokio::task::spawn_blocking(move || execute_query(ast)).await.unwrap()
    }).await;

    match result {
        Ok(res) => match res {
            Ok(records) => {
                let is_star = fields.len() == 1 && fields[0] == "*";
                let headers = if is_star {
                    vec!["name".to_string(), "path".to_string(), "size".to_string(), "type".to_string(), "modified_at".to_string(), "created_at".to_string()]
                } else {
                    fields.clone()
                };

                let fmt_time = |st: &SystemTime| {
                    let dt: DateTime<Local> = (*st).into();
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                };

                match args.output {
                    OutputFormat::Json => {
                        let json_arr: Vec<serde_json::Value> = records.iter().map(|r| {
                            let mut map = serde_json::Map::new();
                            for h in &headers {
                                match h.as_str() {
                                    "name" => { map.insert(h.clone(), serde_json::Value::String(r.name.clone())); },
                                    "path" => { map.insert(h.clone(), serde_json::Value::String(r.path.to_string_lossy().to_string())); },
                                    "size" => { map.insert(h.clone(), serde_json::Value::Number(r.size.into())); },
                                    "type" => { map.insert(h.clone(), serde_json::Value::String(r.type_.clone())); },
                                    "modified_at" => { map.insert(h.clone(), serde_json::Value::String(fmt_time(&r.modified_at))); },
                                    "created_at" => { map.insert(h.clone(), serde_json::Value::String(fmt_time(&r.created_at))); },
                                    _ => {},
                                };
                            }
                            serde_json::Value::Object(map)
                        }).collect();
                        
                        let j = serde_json::to_string_pretty(&json_arr).into_diagnostic()?;
                        println!("{}", j);
                    }
                    OutputFormat::Table => {
                        if records.is_empty() {
                            println!("No records found.");
                        } else {
                            let mut builder = Builder::default();
                            builder.push_record(headers.clone());
                            
                            for r in &records {
                                let mut row = Vec::new();
                                for h in &headers {
                                    match h.as_str() {
                                        "name" => row.push(r.name.clone()),
                                        "path" => row.push(r.path.to_string_lossy().to_string()),
                                        "size" => row.push(r.size.to_string()),
                                        "type" => row.push(r.type_.clone()),
                                        "modified_at" => row.push(fmt_time(&r.modified_at)),
                                        "created_at" => row.push(fmt_time(&r.created_at)),
                                        _ => row.push("".to_string()),
                                    }
                                }
                                builder.push_record(row);
                            }
                            let table = builder.build().to_string();
                            println!("{}", table);
                        }
                    }
                    OutputFormat::Text => {
                        for r in &records {
                            let mut row = Vec::new();
                            for h in &headers {
                                match h.as_str() {
                                    "name" => row.push(r.name.clone()),
                                    "path" => row.push(r.path.to_string_lossy().to_string()),
                                    "size" => row.push(r.size.to_string()),
                                    "type" => row.push(r.type_.clone()),
                                    "modified_at" => row.push(fmt_time(&r.modified_at)),
                                    "created_at" => row.push(fmt_time(&r.created_at)),
                                    _ => row.push("".to_string()),
                                }
                            }
                            println!("{}", row.join("\t"));
                        }
                    }
                }

                if args.stats {
                    let elapsed = start_time.elapsed();
                    println!("\nStats: Success in {:?}", elapsed);
                }
            }
            Err(e) => {
                if args.stats {
                    println!("\nStats: Failed in {:?}", start_time.elapsed());
                }
                miette::bail!("Engine Error: {}", e)
            }
        },
        Err(_) => {
            miette::bail!("Query timed out after {:?}", duration)
        }
    }

    Ok(())
}
