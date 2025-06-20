// test-kind: before-after

fn test() {
    assert!( a, );
    assert_eq!( a, b, );
    assert_ne!( a, b, );
    cfg!( a, x="y", );
    column!( );
    compile_error!( a, );
    concat!( a, b, );
    dbg!( a, b, );
    debug_assert!( a, );
    debug_assert_eq!( a, b, );
    debug_assert_ne!( a, b, );
    env!( a, b, );
    eprint!(a, b, );
    eprintln!(a, b, );
    file!( );
    format!(a, b, );
    format_args!(a, b, );
    include!( a, );
    include_bytes!(a, );
    include_str!(a, );
    is_x86_feature_detected!( a, );
    line!( );
    matches!( a, b, );
    module_path!( );
    option_env!( a, );
    panic!(a, b, );
    print!(a, b, );
    println!(a, b, );
    stringify!( a~ ;b, );
    todo!(a, b, );
    unimplemented!(a, b, );
    unreachable!(a, b, );
    vec!(a, b, );
    write!(a, b, );
    writeln!(a, b, );
}

fn empty_macros() {
    line!(
    // comment
    )
}

thread_local! (
    pub static FOO:
    Cell<u32> = const { Cell::new(1) };
    static BAR:
    RefCell<Vec<f32>> = RefCell::new(vec![1.0]);
);

// :after:

fn test() {
    assert!(a);
    assert_eq!(a, b);
    assert_ne!(a, b);
    cfg!(a, x = "y");
    column!();
    compile_error!(a);
    concat!(a, b);
    dbg!(a, b);
    debug_assert!(a);
    debug_assert_eq!(a, b);
    debug_assert_ne!(a, b);
    env!(a, b);
    eprint!(a, b);
    eprintln!(a, b);
    file!();
    format!(a, b);
    format_args!(a, b);
    include!(a);
    include_bytes!(a);
    include_str!(a);
    is_x86_feature_detected!(a);
    line!();
    matches!(a, b);
    module_path!();
    option_env!(a);
    panic!(a, b);
    print!(a, b);
    println!(a, b);
    stringify!(a~ ;b,);
    todo!(a, b);
    unimplemented!(a, b);
    unreachable!(a, b);
    vec![a, b];
    write!(a, b);
    writeln!(a, b);
}

fn empty_macros() {
    line!(
    // comment
    )
}

thread_local! {
    pub static FOO: Cell<u32> = const { Cell::new(1) };
    static BAR: RefCell<Vec<f32>> = RefCell::new(vec![1.0]);
}
