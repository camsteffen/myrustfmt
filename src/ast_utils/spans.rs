use crate::span::{Span, get_span};
use crate::util::whitespace_utils::expect_first_token_after_whitespace_and_comments;
use rustc_ast::ast;
use rustc_lexer::TokenKind;
use rustc_span::BytePos;

pub fn block_inside_span(block: &ast::Block, source: &str) -> Span {
    let lo = match block.rules {
        ast::BlockCheckMode::Default => block.span.lo() + BytePos(1),
        ast::BlockCheckMode::Unsafe(_) => {
            let block_str = get_span(source, Span::from(block.span));
            let after_unsafe = block_str
                .strip_prefix("unsafe")
                .expect("unsafe block should start with \"unsafe\"");
            let distance_to_bracket = expect_first_token_after_whitespace_and_comments(
                after_unsafe,
                TokenKind::OpenBrace,
            );
            block.span.lo() + BytePos("unsafe".len() as u32 + distance_to_bracket + 1)
        }
    };
    let hi = block.span.hi() - BytePos(1);
    Span { lo, hi }
}
