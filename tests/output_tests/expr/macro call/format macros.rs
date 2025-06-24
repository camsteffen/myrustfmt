// test-kind: before-after
// max-width: 42

fn format_macros() {
    println!("Hello {} and {}, how are you?", name1, name2);
    assert_eq!(x, y, "x and y were not equal, see {}", reason);
    write!( w, concat!( "{a}" ), a=1 );
}

// :after:

fn format_macros() {
    println!(
        "Hello {} and {}, how are you?",
        name1, name2,
    );
    assert_eq!(
        x, y,
        "x and y were not equal, see {}",
        reason,
    );
    write!(w, concat!("{a}"), a = 1);
}
