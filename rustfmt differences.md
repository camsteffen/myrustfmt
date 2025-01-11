* When a closure body begins with `match`, `loop` or a multi-line struct literal,
  the body is wrapped with a block.
* A chain of binary operators of equal precedence are treated as a single chain.
* A dot chain of two elements (e.g. expr.field) is not exempt from chain_width.
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
* Chains may include index operators
* Large expressions in an index operator are broken into a separate line


rustfmt Bugs:
* fn_call_width is reduced by 2 when the last argument overflows into multiple lines
* chain_width is reduced by 1 when the chain ends with `?`
* when function parameters are formatted on multiple lines, max_width is reduced by 4 when placing `{`
* When an import has curly braces, max_width is reduced by 2
* When placing a `{` after `let...else`, max_width is reduced by 1
* When placing a `{` after `if .. =>` where the if-guard has its own line, max_width is reduced by 2