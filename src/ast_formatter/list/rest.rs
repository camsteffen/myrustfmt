use rustc_ast::ast;

#[derive(Clone, Copy)]
pub struct ListRest<'a> {
    pub base: Option<&'a ast::Expr>,
}

impl<'a> ListRest<'a> {
    pub fn from_pat_fields_rest(rest: ast::PatFieldsRest) -> Option<Self> {
        match rest {
            ast::PatFieldsRest::None => None,
            ast::PatFieldsRest::Rest => Some(ListRest { base: None }),
            ast::PatFieldsRest::Recovered(_) => todo!(),
        }
    }
    
    pub fn from_struct_rest(rest: &'a ast::StructRest) -> Option<Self> {
        match rest {
            ast::StructRest::None => None,
            ast::StructRest::Rest(_) => Some(ListRest { base: None }),
            ast::StructRest::Base(expr) => Some(ListRest { base: Some(expr) }),
        }
    }
}
