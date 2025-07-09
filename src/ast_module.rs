use crate::ast_utils::use_tree_order::SortedUseTreeMap;
use crate::macro_args::MacroArgsMap;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use thin_vec::ThinVec;

pub struct AstModule {
    pub attrs: ThinVec<ast::Attribute>,
    pub items: ThinVec<P<ast::Item>>,
    pub macro_args: MacroArgsMap,
    pub sorted_use_trees: SortedUseTreeMap,
    pub spans: ast::ModSpans,
}
