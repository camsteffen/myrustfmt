## Comments

A single-line block comment is allowed anywhere.

A line comment is allowed so long as a newline is allowed in the same place.

In places where a space is expected and a newline is not allowed, line comments and multi-line block comments are not
allowed. If one is encountered, the AST node will not be formatted and an error will be emitted.