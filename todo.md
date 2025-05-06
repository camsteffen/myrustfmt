
Need to distinguish between a bad newline because of a block comment vs. a bad newline that indicates a wrong shape.
The block comment case should not trigger a fallback strategy.
Maybe need a "never breaking space" and a "breakable space".
Or catch errors when writing a breakable space and throw a different error.




TDD every tail usage


error on newline in comments where a space is being written


does max_width have to be an Option?
