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


use more guards instead of closures

enable use_field_init_shorthand by default?
imports_granularity default?
group_imports?
merge_derives? (probably false by default unlike rustfmt)
use_small_heuristics?
