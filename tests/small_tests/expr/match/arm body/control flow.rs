// test-kind: before-after

fn test() {
    match x {
        _ => for x in y {
            z;
        }
        _ => if y {
            z;
        }
        _ => loop {
            break;
        }
        _ => while x {
            y;
        }
    }
}

// :after:

fn test() {
    match x {
        _ => {
            for x in y {
                z;
            }
        }
        _ => {
            if y {
                z;
            }
        }
        _ => {
            loop {
                break;
            }
        }
        _ => {
            while x {
                y;
            }
        }
    }
}
