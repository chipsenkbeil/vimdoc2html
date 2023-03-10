/*
 * 1. Need to be able to query to get
 *     a. Previous sibling node
 *     b. Next sibling node
 *     c. Parent node
 *
 * 2. Keep track of depth with 0 being root
 *
 * 3. For the base text representation of a node (to be modified later)
 *     a. If current node has no children, it is represented as its text
 *     b. If current node is an error, it is represented as its text
 *     c. Otherwise, it is represented by children as text
 *
 * 4. Line is considered noise if
 *     a. Is the first line
 *     b. Is "Type ... to see the table of contents"
 *     c. Is title line of :help pages
 *        "NVIM REFERENCE MANUAL by ..."
 *     d. Is first line of traditional :help page
 *        "*api.txt*   Nvim"
 *     e. Is modeline
 *        "vim:tw=78:ts=8:sw=4:sts=4:et:ft=help:norl:"
 *
 * ---
 *
 * Stateful information between node visits
 *
 * 1. level = depth of node, starting at 0
 * 2. lang_tree = IGNORE
 * 3. headings = list of {
 *        name = header name,
 *        subheadings = list of headings,
 *        tag = tag name where it comes from
 *
 *            a. first *tag* node within heading
 *            b. header name
 *    }
 * 4. opt = {
 *        buf = IGNORE
 *        fname = IGNORE
 *        old = boolean,
 *        indent = number,
 *    }
 * 5. stats = {
 *        first_tags = list of tag name,
 *        parse_errors = list of error node text,
 *        noise_lines = list of noisy lines by content
 *    }
 *
 * ---
 *
 * :: help_file
 *
 *     return ""
 *
 * :: url
 *
 *     return cleaned url in the form
 *
 *     <a href="...">...</a>
 *
 * :: word / uppercase_name
 *
 *     return {text}/{text}
 *
 * :: h1 / h2 / h3
 *
 *     if is noise, return ""
 *
 *     else return "{anchor}{heading}"
 *     where
 *
 *         anchor = <a name="{tagname}"></a>
 *         if there is no tag child within the heading
 *         else anchor = ""
 *
 *         heading = <h2 class="help-heading">{text}</h2>
 *         if kind = "h1"
 *         else heading = <h3 class="help-heading">{text}</h3>
 *
 * :: column_heading / column_name
 *
 *     if HAS_ERROR (node == ERROR/MISSING), return {text}
 *     else return "<div class="help-column_heading">{text}</div>"
 *
 * :: block
 *
 *     if contains only whitespace, return ""
 *     else if opt.old return "<div class="old-help-para">{text}</div>"
 *     else return "<div class="help-para">{text}</div>"
 *
 * :: line
 *
 *     if parent is not code or code block and
 *         line is blank or noise
 *         return ""
 *
 *     else if opt.old and has first child that is column_heading or h1 or h2 or h3
 *         return  "trim({text})"
 *
 *     else
 *         return "{text}\n"
 *
 * :: line_li
 *
 *     if no previous line_li sibling, reset opt.indent = 1
 *     else if previous line_li is indented less, it is the parent and opt.indent += 1
 *     else if previous line_li is indented more, decrement opt.indent -= 1 (min 1)
 *
 *     margin-left (css) = 1.5 * opt.indent (if > 1)
 *     return <div class="help-li" style="margin-left:...">{text}</div>
 */
use super::{FromParser, Parser};
use crate::utils;
use crate::{Context, NodeType, Visitor, SPACE_STRING_JOINER};
use std::ops::{Deref, DerefMut};

/// Newtype [`String`] representing HTML output from a [`Parser`].
pub struct HtmlString(String);

impl From<HtmlString> for String {
    fn from(x: HtmlString) -> Self {
        x.0
    }
}

impl Deref for HtmlString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HtmlString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromParser for HtmlString {
    type Err = ();

    /// Parses into an HTML string.
    fn from_parser(parser: &Parser) -> Result<Self, Self::Err> {
        let mut visitor = HtmlVisitor::new(HtmlVisitorOpt { old: false });

        Ok(HtmlString(visitor.visit_all_named(
            &mut Context {
                src: parser.src(),
                cursor: &mut parser.tree().walk(),
            },
            &SPACE_STRING_JOINER,
        )))
    }
}
