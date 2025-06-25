// test-kind: before-after

foo! ( );
bar! [
         // comment
];
baz!  {   ding
    dong  }

// :after:

foo!();
bar![
    // comment
];
baz! {   ding
    dong  }
