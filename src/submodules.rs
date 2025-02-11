use crate::ast_module::AstModule;
use rustc_ast::ModKind;
use rustc_ast::ast;
use rustc_ast::visit::{self, Visitor};
use rustc_expand::module::DirOwnership;
use rustc_expand::module::ModError;
use rustc_expand::module::default_submod_path;
use rustc_session::parse::ParseSess;
use rustc_span::sym;
use rustc_span::symbol::Ident;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Submodule {
    pub path: PathBuf,
    pub relative: Option<Ident>,
}

/// * `relative` - this is Some("foo") when we're in foo.rs and the corresponding module directory
///   is ./foo/
pub fn get_submodules(
    psess: &ParseSess,
    module: &AstModule,
    path: &Path,
    relative: Option<Ident>,
) -> Vec<Submodule> {
    let dir = path
        .parent()
        .expect("the file path should have a parent")
        .to_path_buf();
    let mut visitor = SubmoduleVisitor {
        psess,
        dir,
        relative,
        submodules: Vec::new(),
    };
    for item in &module.items {
        visitor.visit_item(item);
    }
    visitor.submodules
}

struct SubmoduleVisitor<'psess> {
    psess: &'psess ParseSess,
    dir: PathBuf,
    relative: Option<Ident>,
    submodules: Vec<Submodule>,
}

fn is_mod_inline(kind: &ModKind) -> bool {
    match kind {
        ast::ModKind::Loaded(_, ast::Inline::No, ..) | ast::ModKind::Unloaded => false,
        ast::ModKind::Loaded(_, ast::Inline::Yes, ..) => true,
    }
}

impl Visitor<'_> for SubmoduleVisitor<'_> {
    fn visit_item(&mut self, item: &ast::Item) {
        match &item.kind {
            ast::ItemKind::Mod(_, mod_kind) => {
                let path_from_attr = self.path_from_attr(item);
                if is_mod_inline(mod_kind) {
                    let dir = path_from_attr.unwrap_or_else(|| self.inline_mod_dir(item));
                    let dir_prev = std::mem::replace(&mut self.dir, dir);
                    let relative = self.relative.take();
                    visit::walk_item(self, item);
                    self.relative = relative;
                    self.dir = dir_prev;
                } else {
                    let submodule = if let Some(path) = path_from_attr {
                        let relative = None;
                        Submodule { path, relative }
                    } else { self.find_external_module(item) };
                    self.submodules.push(submodule);
                }
            }
            _ => visit::walk_item(self, item),
        }
    }
}

impl SubmoduleVisitor<'_> {
    // #[path = "..."]
    // https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute
    fn path_from_attr(&self, item: &ast::Item) -> Option<PathBuf> {
        let attr = item.attrs.iter().find(|a| a.has_name(sym::path))?;
        let path = attr.value_str()?;
        Some(self.dir.join(path.as_str()))
    }

    fn find_external_module(&mut self, item: &ast::Item) -> Submodule {
        // todo check for mod cycle
        let mod_path = match default_submod_path(self.psess, item.ident, self.relative, &self.dir) {
            Ok(mod_path) => mod_path,
            Err(e) => self.mod_error(e),
        };
        let DirOwnership::Owned { relative } = mod_path.dir_ownership else {
            unreachable!();
        };
        Submodule {
            path: mod_path.file_path,
            relative,
        }
    }

    fn inline_mod_dir(&self, item: &ast::Item) -> PathBuf {
        let mut dir = self.dir.clone();
        if let Some(relative) = self.relative {
            // we're in foo.rs and found `mod bar {..}`, so go into ./foo/bar
            dir.push(relative.name.as_str());
            dir.push(item.ident.name.as_str());
        } else {
            dir.push(item.ident.name.as_str());
        }
        dir
    }

    fn mod_error(&self, error: ModError) -> ! {
        match error {
            ModError::FileNotFound(ident, _default_path, _secondary_path) => {
                self.psess
                    .dcx()
                    .span_err(ident.span, "file not found for module");
                todo!();
            }
            ModError::MultipleCandidates(ident, default_path, secondary_path) => {
                let msg = format!(
                    "file for module `{ident}` found at both \"{}\" and \"{}\"",
                    default_path.display(),
                    secondary_path.display(),
                );
                self.psess.dcx().span_err(ident.span, msg);
                todo!();
            }
            ModError::ParserError(_) | ModError::CircularInclusion(_) | ModError::ModInBlock(_) => {
                // todo the function never returns these errors, but is there a better way?
                unreachable!()
            }
        }
    }
}
