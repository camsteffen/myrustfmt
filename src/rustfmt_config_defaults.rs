pub struct RustfmtConfig {
    pub array_width: usize,
    pub attr_fn_like_width: usize,
    pub chain_width: usize,
    pub fn_call_width: usize,
    pub short_array_element_width_threshold: usize,
    pub single_line_if_else_max_width: usize,
    pub struct_lit_width: usize,
    pub struct_variant_width: usize,
}

pub const RUSTFMT_CONFIG_DEFAULTS: RustfmtConfig = RustfmtConfig {
    array_width: 60,
    attr_fn_like_width: 70,
    chain_width: 60,
    fn_call_width: 60,
    short_array_element_width_threshold: 10,
    single_line_if_else_max_width: 50,
    struct_lit_width: 18,
    struct_variant_width: 35,
};
