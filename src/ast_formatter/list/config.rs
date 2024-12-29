use crate::config::Config;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

pub trait ListConfig {
    fn overflow_max_first_line_contents_width(&self, _config: &Config) -> Option<usize> {
        None
    }

    fn single_line_block(&self) -> bool {
        false
    }

    fn single_line_max_contents_width(&self) -> Option<usize> {
        None
    }

    fn single_line_reduce_max_width_quirk(&self, _config: &Config) -> usize {
        0
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::No
    }
    
    // fn item_must_have_own_line
}

pub struct DefaultListConfig;
impl ListConfig for DefaultListConfig {}

pub enum ListWrapToFitConfig {
    No,
    Yes { max_element_width: Option<usize> },
}

pub struct ArrayListConfig;
impl ListConfig for ArrayListConfig {
    fn single_line_max_contents_width(&self) -> Option<usize> {
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
    fn overflow_max_first_line_contents_width(&self, config: &Config) -> Option<usize> {
        if config.rustfmt_quirks {
            Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width - 2)
        } else {
            Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)
        }
    }

    fn single_line_max_contents_width(&self) -> Option<usize> {
        Some(RUSTFMT_CONFIG_DEFAULTS.fn_call_width)
    }
}

pub struct ParamListConfig {
    pub single_line_max_contents_width: Option<usize>,
}
impl ListConfig for ParamListConfig {
    fn single_line_max_contents_width(&self) -> Option<usize> {
        self.single_line_max_contents_width
    }
}

pub fn struct_field_list_config(
    single_line_block: bool,
    single_line_max_contents_width: usize,
) -> impl ListConfig {
    pub struct StructFieldListConfig {
        single_line_block: bool,
        single_line_max_contents_width: usize,
    }
    impl ListConfig for StructFieldListConfig {
        fn single_line_block(&self) -> bool {
            self.single_line_block
        }

        fn single_line_max_contents_width(&self) -> Option<usize> {
            Some(self.single_line_max_contents_width)
        }
    }
    StructFieldListConfig {
        single_line_block,
        single_line_max_contents_width,
    }
}
