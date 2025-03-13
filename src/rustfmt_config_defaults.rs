use crate::num::HPos;

// todo delete or what?
#[allow(dead_code)]
pub struct RustfmtConfig {
    pub array_width: HPos,
    pub attr_fn_like_width: HPos,
    pub chain_width: HPos,
    pub fn_call_width: HPos,
    pub short_array_element_width_threshold: HPos,
    pub single_line_if_else_max_width: HPos,
    pub single_line_let_else_max_width: HPos,
    pub struct_lit_width: HPos,
    pub struct_variant_width: HPos,
}

pub const RUSTFMT_CONFIG_DEFAULTS: RustfmtConfig = RustfmtConfig {
    array_width: 60,
    attr_fn_like_width: 70,
    chain_width: 60,
    fn_call_width: 60,
    short_array_element_width_threshold: 10,
    single_line_if_else_max_width: 50,
    single_line_let_else_max_width: 50,
    struct_lit_width: 18,
    struct_variant_width: 35,
};
