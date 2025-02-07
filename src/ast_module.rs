use rustc_ast::ast;
use rustc_ast::ptr::P;
use thin_vec::ThinVec;

pub struct AstModule {
    pub attrs: ThinVec<ast::Attribute>,
    pub items: ThinVec<P<ast::Item>>,
    pub spans: ast::ModSpans,
}
