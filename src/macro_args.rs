use crate::parse::parse_no_errors;
use crate::std_macro::{StdMacro, std_macro};
use rustc_ast::ast;
use rustc_ast::ptr::P;
use rustc_ast::token;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::PResult;
use rustc_parse::MACRO_ARGUMENTS;
use rustc_parse::exp;
use rustc_parse::parser::Parser;
use rustc_parse::parser::Recovery;
use rustc_session::parse::ParseSess;
use rustc_span::BytePos;
use thin_vec::ThinVec;

pub type MacroArgsMap = FxHashMap<BytePos, MacroArgs>;

pub fn mac_call_id(mac_call: &ast::MacCall) -> BytePos {
    mac_call.path.span.lo()
}

#[derive(Default)]
pub struct MacroArgsCollector {
    pub macro_args: MacroArgsMap,
}

// todo parse nested macro calls
impl MacroArgsCollector {
    pub fn expr(&mut self, psess: &ParseSess, expr: &ast::Expr) {
        if let ast::ExprKind::MacCall(mac_call) = &expr.kind {
            self.mac_call(psess, mac_call);
        }
    }

    pub fn stmt(&mut self, psess: &ParseSess, stmt: &ast::Stmt) {
        if let ast::StmtKind::MacCall(mac_call_stmt) = &stmt.kind {
            self.mac_call(psess, &mac_call_stmt.mac);
        }
    }

    fn mac_call(&mut self, psess: &ParseSess, mac_call: &ast::MacCall) {
        if let Some(mac_args) = try_parse_macro_args(psess, mac_call) {
            self.macro_args.insert(mac_call_id(mac_call), mac_args);
        }
    }
}

pub enum MacroArgs {
    /// Same as a function call. Optional trailing comma. Also used for macros with no args.
    ExprList(ThinVec<P<ast::Expr>>),
    Format {
        args: ThinVec<P<ast::Expr>>,
        format_string_pos: u8,
    },
    MetaItemInner(ThinVec<ast::MetaItemInner>),
}

// todo emit an error if we fail to parse a known macro? at least in debug mode
pub fn try_parse_macro_args(psess: &ParseSess, mac_call: &ast::MacCall) -> Option<MacroArgs> {
    let std_macro = std_macro(mac_call)?;
    let tokens = mac_call.args.tokens.clone();
    // todo is MACRO_ARGUMENTS necessary?
    // todo silence errors except in debug mode
    let parser = Parser::new(psess, tokens, MACRO_ARGUMENTS)
        .recovery(Recovery::Forbidden);
    let macro_args = parse_no_errors(parser, |parser| match std_macro {
        StdMacro::Cfg => {
            parse_comma_sep_list(parser, |p| p.parse_meta_item_inner())
                .map(MacroArgs::MetaItemInner)
        }
        StdMacro::ExprList => {
            parse_comma_sep_list(parser, |p| p.parse_expr())
                .map(MacroArgs::ExprList)
        }
        StdMacro::Format { format_string_pos } => {
            parse_comma_sep_list(parser, |p| p.parse_expr())
                .map(|args| {
                    MacroArgs::Format {
                        args,
                        format_string_pos,
                    }
                })
        }
    })
    .ok()?;
    Some(macro_args)
}

fn parse_comma_sep_list<'p, T>(
    parser: &mut Parser<'p>,
    parse: impl for<'a> Fn(&mut Parser<'a>) -> PResult<'a, T>,
) -> PResult<'p, ThinVec<T>> {
    let mut list = ThinVec::new();
    loop {
        if parser.token == token::Eof {
            break;
        }
        let next = parse(parser)?;
        list.push(next);
        if !parser.eat(exp!(Comma)) {
            parser.expect(exp!(Eof))?;
            break;
        }
    }
    Ok(list)
}
