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
        "column" | "compile_error" | "concat" | "dbg" | "env" => StdMacro::ExprList,
        "assert" | "debug_assert" => StdMacro::Format {
            format_string_pos: 1,
        },
        "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => StdMacro::Format {
            format_string_pos: 2,
        },
        "eprint" | "eprintln" | "print" | "println" => StdMacro::Format {
            format_string_pos: 0,
        },
        "cfg" => StdMacro::Cfg,
        _ => return None,
    };
    Some(std_macro)
}
