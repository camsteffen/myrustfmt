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
* Closures can be directly nested in closures without wrapping the nested closure in a block.
  Rustfmt wraps the nested closure in a block if the nested closure is multiple lines and the outer closure args fit in a single line.
* Closure arguments on multiple lines are formatted like other lists, not with visual style.
* When breaking nested function calls into multiple lines, prefers to add breaks towards the outermost call

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


## Implementation Differences

* Formatting functions do not produce `String`s or use `format!`. There is one `String` buffer for the entire file and
  formatting functions push one token at a time into the buffer.
* Constraints are enabled on a section of formatting code to limit the shape of its output. Constraints are checked for
  every token emitted, so they fail early, and they don't need to be checked in many places in code.
* When multiple formatting strategies are possible, usually we just attempt one strategy at a time and continue with the
  first one to not return an error. There are just a few cases where we need to run multiple strategies and compare the
  dimensions of the output.
* There is one code path for handing whitespace and comments. Comments are almost entirely abstracted away from the main
  formatting logic -- outputting spaces and newlines automatically allows for comments.
* There are no hard-coded numbers for the known width of tokens and no math involving such numbers.
* There is no `Rewrite` trait implemented for every node type. Just a lot of explicit functions.
* Formatting does not use an AST visitor.
* Every token of output is checked against the input. The sequence of tokens must be the same unless an exception is
  explicitly made. This guarantees that, if there is a bug, the program will crash before it tries to change your code
  or delete your comments.
