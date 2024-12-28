
pub struct Braces {
    pub start: &'static str,
    pub end: &'static str,
    pub pad: bool,
}

impl Braces {
    pub const ANGLE: &'static Braces = &Braces::new("<", ">", false);
    pub const CURLY: &'static Braces = &Braces::new("{", "}", true);
    pub const CURLY_NO_PAD: &'static Braces = &Braces::new("{", "}", false);
    pub const PARENS: &'static Braces = &Braces::new("(", ")", false);
    pub const PIPE: &'static Braces = &Braces::new("|", "|", false);
    pub const SQUARE: &'static Braces = &Braces::new("[", "]", false);

    const fn new(start: &'static str, end: &'static str, pad: bool) -> Braces {
        Braces { start, end, pad }
    }
}
