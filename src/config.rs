pub struct Config {
    pub max_width: usize,
    pub rustfmt_quirks: bool,
}

pub const DEFAULT_CONFIG: Config = Config { max_width: 100, rustfmt_quirks: true };
