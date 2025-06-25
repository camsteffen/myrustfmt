use crate::Submodule;
use crate::macro_args::MacroArgsMap;
use crate::macro_args::MacroArgsParser;
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
        macro_args: MacroArgsParser {
            psess,
            macro_args: Default::default(),
        },
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
    macro_args: MacroArgsParser<'psess>,
    submodules: Option<SubmoduleCollector>,
}

impl Visitor<'_> for ModuleExtrasVisitor<'_> {
    fn visit_item(&mut self, item: &ast::Item) {
        if let Some(submodules) = &mut self.submodules {
            let close = submodules.visit_item(self.psess, item);
            visit::walk_item(self, item);
            close(self.submodules.as_mut().unwrap());
        } else {
            visit::walk_item(self, item);
        }
    }

    fn visit_mac_call(&mut self, mac_call: &ast::MacCall) {
        self.macro_args.visit_mac_call(mac_call);
        visit::walk_mac(self, mac_call);
    }
}
