pub struct RustfmtConfig {
    pub array_width: usize,
    pub attr_fn_like_width: usize,
    pub chain_width: usize,
    pub fn_call_width: usize,
    pub struct_lit_width: usize,
}

pub const RUSTFMT_CONFIG_DEFAULTS: RustfmtConfig = RustfmtConfig {
    array_width: 10,
    attr_fn_like_width: 70,
    chain_width: 60,
    fn_call_width: 60,
    struct_lit_width: 18,
};
