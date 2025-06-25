use rustc_span::BytePos;

/// Like rustc Span minus all the macro expansion complications
pub struct Span {
    pub lo: BytePos,
    pub hi: BytePos,
}

impl From<rustc_span::Span> for Span {
    fn from(span: rustc_span::Span) -> Span {
        Span {
            lo: span.lo(),
            hi: span.hi(),
        }
    }
}
