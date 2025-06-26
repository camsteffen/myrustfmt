use rustc_ast::ModKind;
use rustc_ast::ast;
use rustc_expand::module::DirOwnership;
use rustc_expand::module::ModError;
use rustc_expand::module::default_submod_path;
use rustc_session::parse::ParseSess;
use rustc_span::sym;
use rustc_span::symbol::Ident;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Submodule {
    pub path: PathBuf,
    pub relative: Option<Ident>,
}

pub struct SubmoduleCollector {
    pub dir: PathBuf,
    pub relative: Option<Ident>,
    pub submodules: Vec<Submodule>,
}

impl SubmoduleCollector {
    #[must_use]
    pub fn visit_item(
        &mut self,
        psess: &ParseSess,
        item: &ast::Item,
    ) -> impl FnOnce(&mut Self) + use<> {
        let mut prev = None;
        match item.kind {
            ast::ItemKind::Mod(_, ident, ref mod_kind) => {
                let path_from_attr = self.path_from_attr(item);
                if is_mod_inline(mod_kind) {
                    let dir = path_from_attr.unwrap_or_else(|| self.inline_mod_dir(ident));
                    let prev_dir = std::mem::replace(&mut self.dir, dir);
                    let prev_relative = self.relative.take();
                    prev = Some((prev_dir, prev_relative));
                } else {
                    let submodule = if let Some(path) = path_from_attr {
                        let relative = None;
                        Submodule { path, relative }
                    } else {
                        self.find_external_module(psess, ident)
                    };
                    self.submodules.push(submodule);
                }
            }
            _ => {}
        }
        |this| {
            if let Some((dir, relative)) = prev {
                this.dir = dir;
                this.relative = relative;
            }
        }
    }

    // #[path = "..."]
    // https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute
    fn path_from_attr(&self, item: &ast::Item) -> Option<PathBuf> {
        let attr = item.attrs.iter().find(|a| a.has_name(sym::path))?;
        let path = attr.value_str()?;
        Some(self.dir.join(path.as_str()))
    }

    fn find_external_module(&mut self, psess: &ParseSess, ident: Ident) -> Submodule {
        // todo check for mod cycle
        let mod_path = match default_submod_path(psess, ident, self.relative, &self.dir) {
            Ok(mod_path) => mod_path,
            Err(e) => self.mod_error(psess, e),
        };
        let DirOwnership::Owned { relative } = mod_path.dir_ownership else {
            unreachable!();
        };
        Submodule {
            path: mod_path.file_path,
            relative,
        }
    }

    fn inline_mod_dir(&self, ident: Ident) -> PathBuf {
        let mut dir = self.dir.clone();
        if let Some(relative) = self.relative {
            // we're in foo.rs and found `mod bar {..}`, so go into ./foo/bar
            dir.push(relative.name.as_str());
            dir.push(ident.name.as_str());
        } else {
            dir.push(ident.name.as_str());
        }
        dir
    }

    fn mod_error(&self, psess: &ParseSess, error: ModError) -> ! {
        match error {
            ModError::FileNotFound(ident, _default_path, _secondary_path) => {
                psess
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
                psess.dcx().span_err(ident.span, msg);
                todo!();
            }
            ModError::ParserError(_) | ModError::CircularInclusion(_) | ModError::ModInBlock(_) => {
                // todo the function never returns these errors, but is there a better way?
                unreachable!()
            }
        }
    }
}

fn is_mod_inline(kind: &ModKind) -> bool {
    match kind {
        ast::ModKind::Loaded(_, ast::Inline::No, ..) | ast::ModKind::Unloaded => false,
        ast::ModKind::Loaded(_, ast::Inline::Yes, ..) => true,
    }
}
