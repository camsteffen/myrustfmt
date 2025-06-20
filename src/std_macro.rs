use rustc_ast::ast;

pub enum StdMacro {
    Cfg,
    ExprList,
    Format { format_string_pos: u8 },
}

pub fn std_macro(mac_call: &ast::MacCall) -> Option<StdMacro> {
    let [segment] = &mac_call.path.segments[..] else {
        return None;
    };
    let std_macro = match segment.ident.as_str() {
        "assert"
        | "assert_eq"
        | "assert_ne"
        | "column"
        | "compile_error"
        | "concat"
        | "dbg"
        | "debug_assert"
        | "debug_assert_eq"
        | "debug_assert_ne"
        | "env" => StdMacro::ExprList,
        "eprint" => StdMacro::Format {
            format_string_pos: 0,
        },
        "cfg" => StdMacro::Cfg,
        _ => return None,
    };
    Some(std_macro)
}
