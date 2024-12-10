use crate::ast_formatter::AstFormatter;
use crate::source_formatter::FormatResult;
use rustc_ast::ast;

impl<'a> AstFormatter<'a> {
    pub fn item(&mut self, item: &ast::Item) -> FormatResult {
        match &item.kind {
            ast::ItemKind::ExternCrate(_) => todo!(),
            ast::ItemKind::Use(_) => todo!(),
            ast::ItemKind::Static(_) => todo!(),
            ast::ItemKind::Const(_) => todo!(),
            ast::ItemKind::Fn(fn_) => self.fn_(fn_, item),
            ast::ItemKind::Mod(_, _) => todo!(),
            ast::ItemKind::ForeignMod(_) => todo!(),
            ast::ItemKind::GlobalAsm(_) => todo!(),
            ast::ItemKind::TyAlias(_) => todo!(),
            ast::ItemKind::Enum(_, _) => todo!(),
            ast::ItemKind::Struct(_, _) => todo!(),
            ast::ItemKind::Union(_, _) => todo!(),
            ast::ItemKind::Trait(_) => todo!(),
            ast::ItemKind::TraitAlias(_, _) => todo!(),
            ast::ItemKind::Impl(_) => todo!(),
            ast::ItemKind::MacCall(_) => todo!(),
            ast::ItemKind::MacroDef(_) => todo!(),
            ast::ItemKind::Delegation(_) => todo!(),
            ast::ItemKind::DelegationMac(_) => todo!(),
        }
    }
}
