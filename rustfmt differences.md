* When a closure body begins with `match`, `loop` or a multi-line struct literal,
  it is wrapped with a block.
* A chain of binary operators of equal precedence are treated as a single chain.
* A dot chain of only two elements (e.g. expr.field) may be wrapped and is not exempt from chain_width.
* A method call with one argument that is a function call is not exempt from fn_call_width.
* When `impl` generics are formatted on multiple lines, the rest of the header is not indented.
* Struct patterns are formatted consistently with struct expressions. Rustfmt gives struct patterns
  a unique format when there is a `..`, the struct can not fit in one line,
  but the fields can fit in one line by themselves.
  ```rust
  let Struct {
    field1, field2, ..
  } = x;
  ```
  This formatter always puts all fields on separate lines if the struct does not fit on one line.
* Large expressions in an index operator are broken into a separate line
* Large expressions in parentheses are broken into a separate line
* A match arm with a guard on a separate line may have its body on one line without a block
* A multi-line if/else in a call argument is wrapped instead of continuing after the parenthesis

Chains
* Chains may include index operators
* Multi-line chains as a match arm body are always wrapped with a block. Rustfmt makes an exception when the chain
  ends in a multi-line method call (this is probably a bug).
* Multi-line chains with no indent as a list item (e.g. array element) are wrapped with a block
* Separate line chains are preferred over overflow if the number of lines is the same
  * Rationale: it's better for higher level structures to use line breaks than more deeply nested structures

rustfmt Bugs:
* fn_call_width is reduced by 2 when the last argument overflows into multiple lines
* chain_width is reduced by 1 when the chain ends with `?`
* match arm width is reduced by 1 when ending with `?`
* when function parameters are formatted on multiple lines, max_width is reduced by 4 when placing `{`
* When an import has curly braces, max_width is reduced by 2
* When placing a `{` after `let...else`, max_width is reduced by 1
* When placing a `{` after `if .. =>` where the if-guard has its own line, max_width is reduced by 2


TODO
* rustfmt seems to shift comments up to the end of the previous line in wrap-to-fit lists
