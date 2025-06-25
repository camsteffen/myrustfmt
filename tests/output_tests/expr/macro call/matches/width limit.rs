// test-kind: before-after

fn test() {
    matches!(aaaaaaaaaaaaaaa, BBBBBBBBBBBBBBBB if ccccccccccccccccccccccc);
    matches!(aaaaaaaaaaaaaaa, BBBBBBBBBBBBBBBB if cccccccccccccccccccccccc);

    matches!(
        aaaaaaaaaaaaaaa,
        BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB if cccccccccc,
    );
    matches!(
        aaaaaaaaaaaaaaa,
        BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB if cccccccccc,
    );
}

// :after:

fn test() {
    matches!(aaaaaaaaaaaaaaa, BBBBBBBBBBBBBBBB if ccccccccccccccccccccccc);
    matches!(
        aaaaaaaaaaaaaaa,
        BBBBBBBBBBBBBBBB if cccccccccccccccccccccccc,
    );

    matches!(
        aaaaaaaaaaaaaaa,
        BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB if cccccccccc,
    );
    matches!(
        aaaaaaaaaaaaaaa,
        BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB
            if cccccccccc,
    );
}
