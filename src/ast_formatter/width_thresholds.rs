use crate::num::HSize;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

pub struct WidthThresholds {
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

pub const WIDTH_THRESHOLDS: WidthThresholds = WidthThresholds {
    array_width: RUSTFMT_CONFIG_DEFAULTS.array_width,
    attr_fn_like_width: RUSTFMT_CONFIG_DEFAULTS.attr_fn_like_width,
    chain_width: RUSTFMT_CONFIG_DEFAULTS.chain_width,
    fn_call_width: RUSTFMT_CONFIG_DEFAULTS.fn_call_width,
    short_array_element_width_threshold: RUSTFMT_CONFIG_DEFAULTS
        .short_array_element_width_threshold,
    single_line_if_else_max_width: RUSTFMT_CONFIG_DEFAULTS.single_line_if_else_max_width,
    single_line_let_else_max_width: RUSTFMT_CONFIG_DEFAULTS.single_line_let_else_max_width,
    struct_lit_width: RUSTFMT_CONFIG_DEFAULTS.struct_lit_width,
    struct_variant_width: RUSTFMT_CONFIG_DEFAULTS.struct_variant_width,
};
