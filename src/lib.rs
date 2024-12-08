#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_builtin_macros;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_expand;
extern crate rustc_parse;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_lexer;
extern crate thin_vec;


pub mod out;
pub mod format_tree;
mod withlexer;
mod withparser;