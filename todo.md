enforce chain width with overflowing closure

this case feels weird, does not match rustfmt
maybe exclude call width for one argument? only if the method name is short, or short distance from margin?
Some(
    aaaaaaaaaaaaaaaaa(bbbbbbbbb, ccccccc),
)

Why isn't fn_call_width applied here?
let errors = Rc::new(
    BufferedErrorEmitter::new(ErrorEmitter::new(path.clone(), config.capture_error_output)),
);

Why does rustfmt wrap this &&?
if let Some(width_limit) = self.width_limit.get() && width_limit.line == line {

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
