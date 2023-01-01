use crate::Parser;

/// Parse a value from a [`Parser`].
pub trait FromParser: Sized {
    type Err;

    fn from_parser(parser: &Parser) -> Result<Self, Self::Err>;
}

mod debug;
mod html;

pub use debug::DebugString;
pub use html::HtmlString;
