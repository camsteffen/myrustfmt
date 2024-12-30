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
            
            pub fn set(&mut self, name: &str, value: &str) {
                match name {
                    $(stringify!($name) => self.$name = value.parse().unwrap(),)*
                    _ => panic!("Invalid config name: {name}"),
                }
            }
        }
    };
}

config! {
    max_width: usize = 100,
    rustfmt_quirks: bool = true,
}