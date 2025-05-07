use crate::num::HSize;

// todo delete or what?
#[allow(dead_code)]
pub struct RustfmtConfig {
    pub array_width: HSize,
    pub attr_fn_like_width: HSize,
    pub chain_width: HSize,
    pub fn_call_width: HSize,
    pub short_array_element_width_threshold: HSize,
    pub single_line_if_else_max_width: HSize,
    pub single_line_let_else_max_width: HSize,
    pub struct_lit_width: HSize,
    pub struct_variant_width: HSize,
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
