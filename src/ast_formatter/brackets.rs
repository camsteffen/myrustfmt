macro_rules! define_brackets {
    ($($name:ident($start:literal, $end:literal, $pad:literal),)*) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Brackets {
            $($name,)*
        }

        impl Brackets {
            pub fn start(self) -> &'static str {
                match self {
                    $(Self::$name => $start,)*
                }
            }

            pub fn end(self) -> &'static str {
                match self {
                    $(Self::$name => $end,)*
                }
            }

            pub fn pad(self) -> bool {
                match self {
                    $(Self::$name => $pad,)*
                }
            }
        }
    };
}

define_brackets! {
    Angle("<", ">", false),
    Curly("{", "}", true),
    CurlyNoPad("{", "}", false),
    Parens("(", ")", false),
    Pipe("|", "|", false),
    Square("[", "]", false),
}
