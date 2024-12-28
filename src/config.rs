macro_rules! config {
    ($($name:ident: $ty:ty = $default:expr,)*) => {
        pub struct Config {
            $(pub $name: $ty,)*
        }

        impl Default for Config {
            fn default() -> Self {
                Config {
                    $($name: $default,)*
                }
            }
        }

        impl Config {
            $(pub fn $name(self, $name: $ty) -> Config {
                Config { $name, ..self }
            })*
        }
    };
}

config! {
    max_width: usize = 100,
    rustfmt_quirks: bool = true,
}
