use crate::Submodule;
use crate::macro_args::MacroArgsCollector;
use crate::macro_args::MacroArgsMap;
use crate::submodules::SubmoduleCollector;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use rustc_ast::visit;
use rustc_ast::visit::Visitor;
use rustc_session::parse::ParseSess;
use rustc_span::Ident;
use std::path::Path;

/// * `relative` - this is Some("foo") when we're in foo.rs and the corresponding module directory
///   is ./foo/
pub fn get_module_extras(
    psess: &ParseSess,
    items: &[P<ast::Item>],
    path: Option<&Path>,
    relative: Option<Ident>,
) -> (Vec<Submodule>, MacroArgsMap) {
    let submodules = path.map(|path| {
        let dir = path
            .parent()
            .expect("the file path should have a parent")
            .to_path_buf();
        SubmoduleCollector {
            dir,
            relative,
            submodules: Vec::new(),
        }
    });
    let mut visitor = ModuleExtrasVisitor {
        psess,
        macro_args: MacroArgsCollector::default(),
        submodules,
    };
    for item in items {
        visitor.visit_item(item);
    }
    (
        visitor.submodules.map_or_default(|s| s.submodules),
        visitor.macro_args.macro_args,
    )
}

struct ModuleExtrasVisitor<'psess> {
    psess: &'psess ParseSess,
    macro_args: MacroArgsCollector,
    submodules: Option<SubmoduleCollector>,
}

impl Visitor<'_> for ModuleExtrasVisitor<'_> {
    fn visit_expr(&mut self, expr: &ast::Expr) {
        self.macro_args.expr(self.psess, expr);
        visit::walk_expr(self, expr);
    }

    fn visit_item(&mut self, item: &ast::Item) {
        let submodules_close = self.submodules.as_mut().map(|s| {
            s.visit_item(self.psess, item)
        });
        visit::walk_item(self, item);
        if let Some(close) = submodules_close {
            close(self.submodules.as_mut().unwrap());
        }
    }

    fn visit_stmt(&mut self, stmt: &ast::Stmt) {
        self.macro_args.stmt(self.psess, stmt);
        visit::walk_stmt(self, stmt);
    }
}
