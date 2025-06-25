use rustc_ast::ast;

pub enum StdMacro {
    Cfg,
    ExprList,
    Format { format_string_pos: u8 },
    Matches,
}

pub fn std_macro(mac_call: &ast::MacCall) -> Option<StdMacro> {
    let [segment] = &mac_call.path.segments[..] else {
        return None;
    };
    let std_macro = match segment.ident.as_str() {
        "cfg" => StdMacro::Cfg,
        "column"
        | "compile_error"
        | "concat"
        | "dbg"
        | "env"
        | "file"
        | "format"
        | "include"
        | "include_bytes"
        | "include_str"
        | "is_x86_feature_detected"
        | "line" => StdMacro::ExprList,
        "eprint" | "eprintln" | "format_args" | "print" | "println" => StdMacro::Format {
            format_string_pos: 0,
        },
        "assert" | "debug_assert" | "write" => StdMacro::Format {
            format_string_pos: 1,
        },
        "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => StdMacro::Format {
            format_string_pos: 2,
        },
        "matches" => StdMacro::Matches,
        _ => return None,
    };
    Some(std_macro)
}
