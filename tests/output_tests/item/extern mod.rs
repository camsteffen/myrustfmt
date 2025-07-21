// test-kind: no-change

extern {}

extern {
    fn test();
    static X: u32;
    type Y = u32;
    macro_call!();
}

extern "C" {
    fn test();
}

unsafe extern {
    pub safe static A: i32;
    pub unsafe static B: i32;
    pub static C: i32;
    pub safe fn d(i: i32);
}
