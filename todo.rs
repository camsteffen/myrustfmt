// test-kind: no-change

// don't wrap the if here
fn test() {
    items.iter().try_for_each(|item| {
        let start = self.out.col();
        let actual_width_limit_end =
            if seen_wide_item {
                width_limit_end
            } else {
                None
            };
    });
}

fn test() {
    match x {
        (x if y, y) => x,
        // should wrap the whole function call
        (Chunk::Number(a), Chunk::Number(b)) => {
            cmp_by_key(a, b, |s| s.parse::<u32>().expect("numeric chunk should be a valid u32"))
        },
    }
}

// #2158
trait Foo {
    type ItRev = <MergingUntypedTimeSeries<SliceSeries<SliceWindow>> as UntypedTimeSeries>::IterRev;
    type IteRev = <MergingUntypedTimeSeries<SliceSeries<SliceWindow>> as UntypedTimeSeries>::IterRev;
}

// #2331
trait MyTrait<AAAAAAAAAAAAAAAAAAAA, BBBBBBBBBBBBBBBBBBBB, CCCCCCCCCCCCCCCCCCCC, DDDDDDDDDDDDDDDDDDDD> {
    fn foo() {}
}

// Trait aliases
trait FooBar =
    Foo
    + Bar;
trait FooBar <A, B, C>=
    Foo
    + Bar;
pub trait FooBar =
    Foo
    + Bar;
pub trait FooBar <A, B, C>=
    Foo
    + Bar;
trait AAAAAAAAAAAAAAAAAA = BBBBBBBBBBBBBBBBBBB + CCCCCCCCCCCCCCCCCCCCCCCCCCCCC + DDDDDDDDDDDDDDDDDD;
pub trait AAAAAAAAAAAAAAAAAA = BBBBBBBBBBBBBBBBBBB + CCCCCCCCCCCCCCCCCCCCCCCCCCCCC + DDDDDDDDDDDDDDDDDD;
trait AAAAAAAAAAAAAAAAAAA = BBBBBBBBBBBBBBBBBBB + CCCCCCCCCCCCCCCCCCCCCCCCCCCCC + DDDDDDDDDDDDDDDDDD;
trait AAAAAAAAAAAAAAAAAA = BBBBBBBBBBBBBBBBBBB + CCCCCCCCCCCCCCCCCCCCCCCCCCCCC + DDDDDDDDDDDDDDDDDDD;
trait AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA<A, B, C, D, E> = FooBar;
trait AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA<A, B, C, D, E> = FooBar;
#[rustfmt::skip]
trait FooBar = Foo
    + Bar;

// #2637
auto trait Example {}
pub auto trait PubExample {}
pub unsafe auto trait PubUnsafeExample {}

// #3006
trait Foo<'a> {
    type Bar<  'a  >;
}

impl<'a> Foo<'a> for i32 {
    type Bar<  'a  > = i32;
}

// #3092
pub mod test {
    pub trait ATraitWithALooongName {}
    pub trait ATrait
        :ATraitWithALooongName + ATraitWithALooongName + ATraitWithALooongName + ATraitWithALooongName
{
}
}

// Trait aliases with where clauses.
trait A = where for<'b> &'b Self: Send;

trait B = where for<'b> &'b Self: Send + Clone + Copy + SomeTrait + AAAAAAAA + BBBBBBB + CCCCCCCCCC;
trait B = where for<'b> &'b Self: Send + Clone + Copy + SomeTrait + AAAAAAAA + BBBBBBB + CCCCCCCCCCC;
trait B = where
    for<'b> &'b Self:
Send + Clone + Copy + SomeTrait + AAAAAAAA + BBBBBBB + CCCCCCCCCCCCCCCCCCCCCCC;
trait B = where
    for<'b> &'b Self:
Send + Clone + Copy + SomeTrait + AAAAAAAA + BBBBBBB + CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC;

trait B = where
    for<'b> &'b Self:
Send
    + Clone
    + Copy
    + SomeTrait
    + AAAAAAAA
    + BBBBBBB
    + CCCCCCCCC
    + DDDDDDD
    + DDDDDDDD
    + DDDDDDDDD
    + EEEEEEE;

trait A<'a, 'b, 'c> = Debug<T> + Foo where for<'b> &'b Self: Send;

trait B<'a, 'b, 'c> = Debug<T> +Foo
where for<'b> &'b Self:
Send
    + Clone
    + Copy
    + SomeTrait
    + AAAAAAAA
    + BBBBBBB
    + CCCCCCCCC
    + DDDDDDD;

trait B<'a, 'b, 'c,T> = Debug<'a, T> where for<'b> &'b Self:
Send
    + Clone
    + Copy
    + SomeTrait
    + AAAAAAAA
    + BBBBBBB
    + CCCCCCCCC
    + DDDDDDD
    + DDDDDDDD
    + DDDDDDDDD
    + EEEEEEE;

trait Visible {
    pub const C: i32;
    pub type T;
    pub fn f();
    pub fn g() {}
}
