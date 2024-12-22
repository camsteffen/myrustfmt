use crate::config::Config;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

pub trait ListConfig {
    fn single_line_block(&self) -> bool {
        false
    }

    fn single_line_reduce_max_width(&self, _config: &Config) -> usize {
        0
    }

    fn single_line_max_contents_width(&self) -> Option<usize> {
        None
    }

    fn overflow_max_first_line_contents_width(&self, _config: &Config) -> Option<usize> {
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

