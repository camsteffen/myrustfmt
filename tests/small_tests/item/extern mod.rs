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
