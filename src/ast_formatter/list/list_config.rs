use crate::num::HPos;
use crate::rustfmt_config_defaults::RUSTFMT_CONFIG_DEFAULTS;

pub trait ListConfig {
    fn force_trailing_comma(&self) -> bool {
        false
    }

    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::No
    }
}

pub struct DefaultListConfig;
impl ListConfig for DefaultListConfig {}

pub enum ListWrapToFitConfig {
    No,
    Yes { max_element_width: Option<HPos> },
}

pub struct ArrayListConfig;
impl ListConfig for ArrayListConfig {
    fn wrap_to_fit() -> ListWrapToFitConfig {
        ListWrapToFitConfig::Yes {
            max_element_width: Some(RUSTFMT_CONFIG_DEFAULTS.short_array_element_width_threshold),
        }
    }
}

pub struct TupleListConfig {
    pub len: usize,
}
impl ListConfig for TupleListConfig {
    fn force_trailing_comma(&self) -> bool {
        self.len == 1
    }
}
