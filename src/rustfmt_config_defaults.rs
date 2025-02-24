// todo delete or what?
#[allow(dead_code)]
pub struct RustfmtConfig {
    pub array_width: u32,
    pub attr_fn_like_width: u32,
    pub chain_width: u32,
    pub fn_call_width: u32,
    pub short_array_element_width_threshold: u32,
    pub single_line_if_else_max_width: u32,
    pub single_line_let_else_max_width: u32,
    pub struct_lit_width: u32,
    pub struct_variant_width: u32,
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
