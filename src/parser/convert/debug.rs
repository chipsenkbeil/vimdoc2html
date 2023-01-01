use super::{FromParser, Parser};
use crate::utils;
use crate::{visitor, Context, NodeExt, Visitor, NEWLINE_STRING_JOINER};
use std::ops::{Deref, DerefMut};

/// Newtype [`String`] representing debug output from a [`Parser`].
pub struct DebugString(String);

impl From<DebugString> for String {
    fn from(x: DebugString) -> Self {
        x.0
    }
}

impl Deref for DebugString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DebugString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromParser for DebugString {
    type Err = ();

    /// Parses into a debug string.
    fn from_parser(parser: &Parser) -> Result<Self, Self::Err> {
        let mut visitor = visitor!(|_this, ctx| -> String {
            let node = ctx.node();
            let depth = node.depth();
            let node_text = ctx.node_raw_text();
            let is_too_long = node_text.len() > 10;

            format!(
                "{}Kind: {:?} [Row:{}, Col:{}] - [Row:{}, Col:{}] = {}",
                " ".repeat(depth * 4),
                node.kind(),
                node.start_position().row,
                node.start_position().column,
                node.end_position().row,
                node.end_position().column,
                if is_too_long {
                    format!("{:?} [trimmed]", &utils::truncate_str(node_text, 10))
                } else {
                    format!("{node_text:?}")
                },
            )
        });

        Ok(DebugString(visitor.visit_all_named(
            &mut Context {
                src: parser.src(),
                cursor: &mut parser.tree().walk(),
            },
            &NEWLINE_STRING_JOINER,
        )))
    }
}
