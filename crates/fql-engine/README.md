# fql-engine

The core execution engine for FQL. It takes an AST (from `fql-ast`) and evaluates it against the real filesystem.

Uses `jwalk` for extremely fast, highly-parallelized recursive directory traversal.
