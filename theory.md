## Lists

When the width of a list of things goes above a certain threshold, the list becomes more readable to read vertically.
A list of one item should not be considered a list.
A list of two items should not be considered a list.
A dot chain is a list where in `a.b`, `a` is the first item in the list.


The dimensions of a portion of code cannot be measured except by formatting it along with surrounding code.

## Closures

When a closure body is multiple lines, it is generally preferred to surround the expression with a block.
Exceptions to this rule:
 * Block expressions like `const {}` or `unsafe {}`
 * `match` expressions

Rationale: A closure has a significant effect on the semantics of the code contained by it (similar to `loop` or other
control flow constructs).
So the closure header deserves to have its own line.
Especially when there are one or more arguments, it seems appropriate to separate the header from the body.
It might seem acceptable to not add a block in simpler cases, but this is tricky line to draw, so we err on the side of
adding a block for simplicity and consistency.

Rationale: `match` is often used as a closure body expression where the scrutinee comes from the closure argument(s).
Having the `match` start on the same line as the closure arguments is natural to read in these cases.
This benefit is deemed strong enough to create a special case for `match` expressions.

lists, struct literals, and function calls should all be treated the same. Adding a block to all of these does not seem
desirable. Not adding a block creates a difficult scenario where we don't want a closure with a function call that
overflows.
