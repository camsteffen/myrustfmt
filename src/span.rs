use rustc_span::BytePos;
use rustc_span::Pos;

/// Like rustc Span minus all the macro expansion complications
pub struct Span {
    pub lo: BytePos,
    pub hi: BytePos,
}

impl Span {
    pub fn split(self, other: Span) -> (Span, Span) {
        assert!(
            other.lo >= self.lo && other.hi <= self.hi,
            "tried to split span with another span that is not within it",
        );
        (
            Span {
                lo: self.lo,
                hi: other.lo,
            },
            Span {
                lo: other.hi,
                hi: self.hi,
            },
        )
    }
}

impl From<rustc_span::Span> for Span {
    fn from(span: rustc_span::Span) -> Span {
        Span {
            lo: span.lo(),
            hi: span.hi(),
        }
    }
}

pub fn get_span(source: &str, span: Span) -> &str {
    &source[span.lo.to_usize()..span.hi.to_usize()]
}
