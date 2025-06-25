use crate::ast_formatter::brackets::Brackets;
use rustc_ast::ast;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StdMacro {
    Cfg,
    FnLike,
    Format { format_string_pos: u8 },
    Matches,
    Vec,
}

impl StdMacro {
    pub fn brackets(self) -> Brackets {
        match self {
            StdMacro::Cfg | StdMacro::FnLike | StdMacro::Format { .. } | StdMacro::Matches => {
                Brackets::Parens
            }
            StdMacro::Vec => Brackets::Square,
        }
    }
}

pub fn std_macro(mac_call: &ast::MacCall) -> Option<StdMacro> {
    let [segment] = &mac_call.path.segments[..] else {
        return None;
    };
    // macros with no arguments are not here since they are handled generically
    let std_macro = match segment.ident.as_str() {
        "cfg" => StdMacro::Cfg,
        "compile_error"
        | "concat"
        | "dbg"
        | "env"
        | "file"
        | "format"
        | "include"
        | "include_bytes"
        | "include_str"
        | "is_x86_feature_detected"
        | "line"
        | "module_path"
        | "option_env" => StdMacro::FnLike,
        "eprint"
        | "eprintln"
        | "format_args"
        | "panic"
        | "print"
        | "println"
        | "todo"
        | "unimplemented"
        | "unreachable" => StdMacro::Format {
            format_string_pos: 0,
        },
        "assert" | "debug_assert" | "write" | "writeln" => StdMacro::Format {
            format_string_pos: 1,
        },
        "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => StdMacro::Format {
            format_string_pos: 2,
        },
        "matches" => StdMacro::Matches,
        "vec" => StdMacro::Vec,
        _ => return None,
    };
    Some(std_macro)
}
