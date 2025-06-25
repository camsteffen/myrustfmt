use crate::parse::parse_no_errors;
use crate::std_macro::{StdMacro, std_macro};
use rustc_ast::ast;
use rustc_ast::ptr::P;
use rustc_ast::token;
use rustc_ast::visit::Visitor;
use rustc_ast::visit::walk_list;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::PResult;
use rustc_parse::MACRO_ARGUMENTS;
use rustc_parse::exp;
use rustc_parse::parser::CommaRecoveryMode;
use rustc_parse::parser::Parser;
use rustc_parse::parser::RecoverColon;
use rustc_parse::parser::RecoverComma;
use rustc_parse::parser::Recovery;
use rustc_session::parse::ParseSess;
use rustc_span::BytePos;
use thin_vec::ThinVec;

pub type MacroArgsMap = FxHashMap<BytePos, MacroArgs>;

pub fn mac_call_id(mac_call: &ast::MacCall) -> BytePos {
    mac_call.path.span.lo()
}

pub struct MacroArgsParser<'a> {
    pub psess: &'a ParseSess,
    pub macro_args: MacroArgsMap,
}

impl Visitor<'_> for MacroArgsParser<'_> {
    fn visit_mac_call(&mut self, mac_call: &ast::MacCall) {
        if let Some(mac_args) = try_parse_macro_args(self.psess, mac_call) {
            // recursively walk parsed macro args for nested macro calls
            match &mac_args {
                MacroArgs::Cfg(_) => {}
                MacroArgs::ExprList(args) | MacroArgs::Format { args, .. } => {
                    walk_list!(self, visit_expr, args);
                }
                MacroArgs::Matches(expr, pat) => {
                    self.visit_expr(expr);
                    self.visit_pat(pat);
                }
            }
            self.macro_args.insert(mac_call_id(mac_call), mac_args);
        }
    }
}

pub enum MacroArgs {
    Cfg(ThinVec<ast::MetaItemInner>),
    /// Same as a function call. Optional trailing comma. Also used for macros with no args.
    ExprList(ThinVec<P<ast::Expr>>),
    Format {
        args: ThinVec<P<ast::Expr>>,
        format_string_pos: u8,
    },
    Matches(P<ast::Expr>, P<ast::Pat>),
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
            parse_comma_sep_list(parser, Parser::parse_meta_item_inner)
                .map(MacroArgs::Cfg)
        }
        StdMacro::ExprList => {
            parse_comma_sep_list(parser, Parser::parse_expr)
                .map(MacroArgs::ExprList)
        }
        StdMacro::Format { format_string_pos } => {
            parse_comma_sep_list(parser, Parser::parse_expr)
                .map(|args| {
                    MacroArgs::Format {
                        args,
                        format_string_pos,
                    }
                })
        }
        StdMacro::Matches => parse_matches(parser),
    })
    .ok()?;
    Some(macro_args)
}

fn parse_comma_sep_list<'p, T>(
    parser: &mut Parser<'p>,
    parse: fn(&mut Parser<'p>) -> PResult<'p, T>,
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

fn parse_matches<'p>(parser: &mut Parser<'p>) -> PResult<'p, MacroArgs> {
    let expr = parser.parse_expr()?;
    parser.expect(exp!(Comma))?;
    let pat = parser.parse_pat_allow_top_guard(
        None,
        RecoverComma::No,
        RecoverColon::No,
        CommaRecoveryMode::EitherTupleOrPipe,
    )?;
    let _ = parser.eat(exp!(Comma));
    parser.expect(exp!(Eof))?;
    Ok(MacroArgs::Matches(expr, pat))
}
