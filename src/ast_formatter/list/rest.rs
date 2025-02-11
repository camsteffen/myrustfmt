use rustc_ast::ast;

#[derive(Clone, Copy)]
pub enum ListRest<'a> {
    None,
    Rest,
    Base(&'a ast::Expr),
}

impl From<ast::PatFieldsRest> for ListRest<'static> {
    fn from(rest: ast::PatFieldsRest) -> Self {
        match rest {
            ast::PatFieldsRest::None => ListRest::None,
            ast::PatFieldsRest::Rest => ListRest::Rest,
            ast::PatFieldsRest::Recovered(_) => todo!(),
        }
    }
}

impl<'a> From<&'a ast::StructRest> for ListRest<'a> {
    fn from(rest: &'a ast::StructRest) -> Self {
        match rest {
            ast::StructRest::None => ListRest::None,
            ast::StructRest::Rest(_) => ListRest::Rest,
            ast::StructRest::Base(expr) => ListRest::Base(expr),
        }
    }
}
