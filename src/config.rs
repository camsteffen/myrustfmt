use crate::num::HPos;

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
            pub fn set_str(&mut self, name: &str, value: &str) {
                match name {
                    $(stringify!($name) => self.$name = value.parse().unwrap(),)*
                    _ => panic!("Invalid config name: {name}"),
                }
            }

            $(pub fn $name(self, $name: $ty) -> Config {
                Config { $name, ..self }
            })*
        }
    };
}

config! {
    max_width: HPos = 100,
}
