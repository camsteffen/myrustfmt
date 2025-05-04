// test-kind: before-after

fn test() {
    let ast::UseTreeKind::Nested { ref items, span: _ } = x;
    let ast::UseTreeKind::Nested { ref itemsz, span: _ } = x;
}

// :after:

fn test() {
    let ast::UseTreeKind::Nested { ref items, span: _ } = x;
    let ast::UseTreeKind::Nested {
        ref itemsz,
        span: _,
    } = x;
}
