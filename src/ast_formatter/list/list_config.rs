use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

pub trait ListConfig {
    fn force_trailing_comma(&self) -> bool {
        false
    }

    fn single_line_max_contents_width(&self) -> Option<u32> {
        None
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::No
    }
}

pub struct DefaultListConfig;
impl ListConfig for DefaultListConfig {}

pub enum ListWrapToFitConfig {
    No,
    Yes { max_element_width: Option<u32> },
}

pub struct ArrayListConfig;
impl ListConfig for ArrayListConfig {
    fn single_line_max_contents_width(&self) -> Option<u32> {
        Some(RUSTFMT_CONFIG_DEFAULTS.array_width)
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::Yes {
            max_element_width: Some(RUSTFMT_CONFIG_DEFAULTS.short_array_element_width_threshold),
        }
    }
}

pub struct CallParamListConfig;

impl ListConfig for CallParamListConfig {
    fn single_line_max_contents_width(&self) -> Option<u32> {
        Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)
    }
}

pub struct ParamListConfig {
    pub single_line_max_contents_width: Option<u32>,
}
impl ListConfig for ParamListConfig {
    fn single_line_max_contents_width(&self) -> Option<u32> {
        self.single_line_max_contents_width
    }
}

pub struct TupleListConfig {
    pub len: usize,
    pub single_line_max_contents_width: Option<u32>,
}
impl ListConfig for TupleListConfig {
    fn force_trailing_comma(&self) -> bool {
        self.len == 1
    }
    fn single_line_max_contents_width(&self) -> Option<u32> {
        self.single_line_max_contents_width
    }
}

pub fn struct_field_list_config(single_line_max_contents_width: u32) -> impl ListConfig {
    pub struct StructFieldListConfig {
        single_line_max_contents_width: u32,
    }
    impl ListConfig for StructFieldListConfig {
        fn single_line_max_contents_width(&self) -> Option<u32> {
            Some(self.single_line_max_contents_width)
        }
    }
    StructFieldListConfig {
        single_line_max_contents_width,
    }
}
