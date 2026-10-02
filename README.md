# FQL - Filesystem Query Language

FQL is a high-performance Rust-based CLI and library for querying your filesystem using SQL-like syntax. It allows you to select, filter, and inspect files exactly as you would rows in a database.

## Workspace Crates

- **[fql](./crates/fql)**: The main Command Line Interface (CLI).
- **[fql-server](./crates/fql-server)**: The HTTP REST API server mode.
- **[fql-engine](./crates/fql-engine)**: The core execution engine handling filesystem traversal (via `jwalk`).
- **[fql-parser](./crates/fql-parser)**: The custom recursive-descent parser and lexer (`logos`) for the FQL dialect.
- **[fql-ast](./crates/fql-ast)**: The Abstract Syntax Tree (AST) defining the structure of FQL queries.
