use crate::Submodule;
use crate::ast_utils::use_tree_order::{SortedUseTreeMap, use_tree_order};
use crate::macro_args::MacroArgsMap;
use crate::macro_args::MacroArgsParser;
use crate::submodules::SubmoduleCollector;
use rustc_ast::ast;
use rustc_ast::ptr::P;
use rustc_ast::visit;
use rustc_ast::visit::Visitor;
use rustc_data_structures::fx::FxHashMap;
use rustc_session::parse::ParseSess;
use rustc_span::Ident;
use std::path::Path;

pub struct ModuleExtras {
    pub macro_args: MacroArgsMap,
    pub sorted_use_trees: SortedUseTreeMap,
    pub submodules: Vec<Submodule>,
}

/// * `relative` - this is Some("foo") when we're in foo.rs and the corresponding module directory
///   is ./foo/
pub fn get_module_extras(
    psess: &ParseSess,
    items: &[P<ast::Item>],
    path: Option<&Path>,
    relative: Option<Ident>,
) -> ModuleExtras {
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
        sorted_use_trees: FxHashMap::default(),
        submodules,
    };
    for item in items {
        visitor.visit_item(item);
    }
    ModuleExtras {
        macro_args: visitor.macro_args.macro_args,
        sorted_use_trees: visitor.sorted_use_trees,
        submodules: visitor.submodules.map_or_default(|s| s.submodules),
    }
}

struct ModuleExtrasVisitor<'psess> {
    psess: &'psess ParseSess,
    macro_args: MacroArgsParser<'psess>,
    sorted_use_trees: SortedUseTreeMap,
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

    fn visit_use_tree(&mut self, use_tree: &ast::UseTree) {
        // must walk first since ordering of this tree might depend on nested trees
        visit::walk_use_tree(self, use_tree);

        if let ast::UseTreeKind::Nested { items, span } = &use_tree.kind {
            if items.len() > 1 {
                let mut sorted = Vec::from_iter(0..items.len());
                sorted.sort_by(|&a, &b| {
                    use_tree_order(&items[a].0, &items[b].0, &self.sorted_use_trees)
                });
                self.sorted_use_trees.insert(span.lo(), sorted);
            }
        }
    }
}
