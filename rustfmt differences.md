* rustfmt does not apply chain_width to chains with one chained item like `expr.method()`
* rustfmt applies a special formatting to struct patterns that do not fit on one line and have a `..`

Example:

    let MyStruct {
        field1, field2, field3, ..
    } = expr;