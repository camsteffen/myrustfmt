enforce chain width with overflowing closure

this case feels weird, does not match rustfmt
maybe exclude call width for one argument? only if the method name is short, or short distance from margin?
Some(
    aaaaaaaaaaaaaaaaa(bbbbbbbbb, ccccccc),
)

std macros

binary expression width limit?

block statements must be vertical

allow single line closure with return type

disallow nested chain overflow? Or just prefer vertical chain to prevent overflow in general.
let source = Arc::clone(source_file.src.as_ref().expect(
    "the SourceFile should have src",
));

TDD every tail usage

have an explicit rustfmt replacement mode that works with cargo fmt;
otherwise don't support the extra parameters
