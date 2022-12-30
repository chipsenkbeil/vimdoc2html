use crate::types::*;
use std::io;
use std::str::Utf8Error;
use tree_sitter::{Node, Point, TreeCursor};

/// Represents the parser of vimdoc that leverages treesitter to construct a tree and then
/// translates the tree into an intermediate series of Rust structs representing vimdoc.
pub struct Parser {
    /// Content loaded into memory and being parsed.
    src: String,

    /// Tree representing parsed tree sitter of `src`.
    tree: tree_sitter::Tree,
    // Used to determine what to do when errors are encountered.
    // rules: ParserRules,
}

impl Parser {
    /// Loads a new parser to process the `src` using the vimdoc language powered by
    /// [`tree_sitter_vimdoc`].
    pub fn load_vimdoc<R: io::Read>(src: R) -> io::Result<Self> {
        let language = tree_sitter_vimdoc::language();
        Self::load(src, language)
    }

    /// Loads a new parser to process the `src` using the given `language`.
    pub fn load<R: io::Read>(src: R, language: tree_sitter::Language) -> io::Result<Self> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(language).unwrap();
        let src = std::io::read_to_string(src)?;
        let tree = parser
            .parse(&src, None)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to parse vimdoc"))?;

        Ok(Self { src, tree })
    }

    /// Parse the previously-loaded `src` as a [`HelpFile`].
    pub fn parse(&self) -> Result<HelpFile, ParseError> {
        HelpFile::parse(self.src(), &mut self.tree().walk())
    }

    /// Returns a reference to the souce being parsed.
    pub fn src(&self) -> &str {
        &self.src
    }

    /// Returns a reference to the raw tree representing the source.
    pub fn tree(&self) -> &tree_sitter::Tree {
        &self.tree
    }

    /// Generates a debug string for the raw tree representing the source.
    pub fn to_tree_debug_string(&self) -> String {
        let mut output = String::new();

        fn parent_cnt(node: &tree_sitter::Node) -> usize {
            match node.parent() {
                Some(node) => 1 + parent_cnt(&node),
                None => 0,
            }
        }

        for node in
            tree_sitter_traversal::traverse_tree(&self.tree, tree_sitter_traversal::Order::Pre)
        {
            if node.is_named() {
                let depth = parent_cnt(&node);
                let node_text = node.utf8_text(self.src().as_bytes()).unwrap();
                let is_too_long = node_text.len() > 10;

                output.push_str(&format!(
                    "{}Kind: {:?} [Row:{}, Col:{}] - [Row:{}, Col:{}] = {}\n",
                    " ".repeat(depth * 4),
                    node.kind(),
                    node.start_position().row,
                    node.start_position().column,
                    node.end_position().row,
                    node.end_position().column,
                    if is_too_long {
                        format!(
                            "{:?} [trimmed]",
                            &node_text[..floor_char_boundary(node_text, 10)]
                        )
                    } else {
                        format!("{node_text:?}")
                    },
                ));
            }
        }

        output
    }
}

/// From https://doc.rust-lang.org/src/core/str/mod.rs.html#258
fn floor_char_boundary(s: &str, index: usize) -> usize {
    #[inline]
    const fn is_utf8_char_boundary(b: u8) -> bool {
        // This is bit magic equivalent to: b < 128 || b >= 192
        (b as i8) >= -0x40
    }

    if index >= s.len() {
        s.len()
    } else {
        let lower_bound = index.saturating_sub(3);
        let new_index = s.as_bytes()[lower_bound..=index]
            .iter()
            .rposition(|b| is_utf8_char_boundary(*b));

        // SAFETY: we know that the character boundary will be within four bytes
        unsafe { lower_bound + new_index.unwrap_unchecked() }
    }
}

/// Retrieves and returns the text representing by the node using the `src`.
#[inline]
fn node_text<'a>(src: &'a str, node: Node) -> &'a str {
    node.utf8_text(src.as_bytes()).unwrap()
}

#[derive(Clone, Debug)]
pub enum InvalidNode {
    Error { start: Point, details: String },
    Missing { start: Point, details: String },
}

impl InvalidNode {
    pub fn from_node_ref(node: &Node) -> Option<Self> {
        if node.is_error() {
            return Some(Self::Error {
                start: node.start_position(),
                details: node.to_sexp(),
            });
        }

        if node.is_missing() {
            return Some(Self::Missing {
                start: node.start_position(),
                details: node.to_sexp(),
            });
        }

        None
    }

    pub fn start(&self) -> Point {
        match self {
            Self::Error { start, .. } => *start,
            Self::Missing { start, .. } => *start,
        }
    }

    pub fn details(&self) -> &str {
        match self {
            Self::Error { details, .. } => details,
            Self::Missing { details, .. } => details,
        }
    }

    pub fn report(&self) {
        eprintln!(
            "Encountered invalid node @ {}: {}",
            self.start(),
            self.details()
        );
    }
}

#[derive(Debug)]
pub enum ParseError {
    MissingField {
        start: Point,
        name: &'static str,
        node_kind: String,
    },
    TooManyChildren {
        start: Point,
        expected: usize,
        actual: usize,
        node_kind: String,
    },
    TypeError {
        start: Point,
        expected: String,
        actual: String,
    },
    Utf8Error {
        err: Utf8Error,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField {
                start,
                name,
                node_kind,
            } => {
                write!(f, "[{} @ {start}] Missing {name}", node_kind)
            }
            Self::TooManyChildren {
                start,
                expected,
                actual,
                node_kind,
            } => {
                write!(
                    f,
                    "[{node_kind} @ {start}] Expected {expected} named children, but had {actual}",
                )
            }
            Self::TypeError {
                start,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "[@ {start}] Expected {expected}, but was actually {actual}"
                )
            }
            Self::Utf8Error { err } => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<ParseError> for io::Error {
    fn from(x: ParseError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, x)
    }
}

impl From<Utf8Error> for ParseError {
    fn from(x: Utf8Error) -> Self {
        Self::Utf8Error { err: x }
    }
}

macro_rules! concat_kinds {
    ($kind:ident, $($rest:ident),* $(,)?) => {
        concat!(stringify!($kind), $(" or ", stringify!($rest)),*)
    };
}

macro_rules! parse {
    (
        $name:ident,
        $($kind:ident = |$src:ident, $cursor:ident| $body:expr),+ $(,)?
    ) => {
        impl<'src, 'tree> $name<'src> {
            pub fn parse(
                src: &'src str,
                cursor: &'tree mut TreeCursor,
            ) -> Result<$name<'src>, ParseError> {
                parse!(@body src, cursor, $($kind = |$src, $cursor| $body),+)
            }
        }
    };
    ($name:ident, $kind:ident = @many($field:ident = $children:ident)) => {
        parse! { $name, $kind = |src, cursor| {
            let mut children = Vec::new();

            if cursor.goto_first_child() {
                loop {
                    let node = cursor.node();

                    if let Some(invalid_node) = InvalidNode::from_node_ref(&node) {
                        invalid_node.report();
                    } else if node.is_named() {
                        let child = $children::parse(src.as_ref(), cursor)?;
                        children.push(child);
                    }

                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }

            $name { $field: children }
        } }
    };
    ($name:ident, $kind:ident = @single($child_field:ident = $child_struct:ident)) => {
        parse! { $name, $kind = |src, cursor| {
            let mut $child_field = None;
            let node = cursor.node();

            if cursor.goto_first_child() {
                let mut cnt = 0;
                loop {
                    let node = cursor.node();
                    if let Some(invalid_node) = InvalidNode::from_node_ref(&node) {
                        invalid_node.report();
                    } else if node.is_named() {
                        cnt += 1;
                        if cnt == 1 {
                            let child = $child_struct::parse(src.as_ref(), cursor)?;
                            $child_field = Some(child);
                        }
                    }

                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }

                cursor.goto_parent();

                if cnt > 1 {
                    return Err(ParseError::TooManyChildren {
                        start: node.start_position(),
                        expected: 1,
                        actual: cnt,
                        node_kind: node.kind().to_string(),
                    });
                }
            }

            if $child_field.is_none() {
                return Err(ParseError::MissingField {
                    start: node.start_position(),
                    name: stringify!($child_field),
                    node_kind: node.kind().to_string(),
                });
            }

            $name {
                $child_field: $child_field.unwrap(),
            }
        } }
    };
    (
        @body
        $_src:ident,
        $_cursor:ident,
        $($kind:ident = |$src:ident, $cursor:ident| $body:expr),+ $(,)?
    ) => {{
        let node = $_cursor.node();

        match node.kind() {
            $(
                stringify!($kind) => {
                    let $src = $_src;
                    let $cursor = $_cursor;
                    Ok($body)
                }
            )+

            _ => {
                let expected = concat_kinds!($($kind,)+);
                Err(ParseError::TypeError {
                    start: node.start_position(),
                    expected: expected.to_string(),
                    actual: node.kind().to_string(),
                })
            }
        }
    }};
}

parse!(HelpFile, help_file = @many(children = Block));

parse!(Block, block = @many(children = BlockChild));

parse!(
    BlockChild,
    line = |src, cursor| BlockChild::Line(Line::parse(src, cursor)?),
    line_li = |src, cursor| BlockChild::LineLi(LineLi::parse(src, cursor)?),
);

parse!(Line, line = @many(children = LineChild));

parse!(
    LineChild,
    argument = |src, cursor| LineChild::Argument(Argument::parse(src, cursor)?),
    codeblock = |src, cursor| LineChild::Codeblock(Codeblock::parse(src, cursor)?),
    codespan = |src, cursor| LineChild::Codespan(Codespan::parse(src, cursor)?),
    column_heading = |src, cursor| LineChild::ColumnHeading(ColumnHeading::parse(src, cursor)?),
    h1 = |src, cursor| LineChild::H1(H1::parse(src, cursor)?),
    h2 = |src, cursor| LineChild::H2(H2::parse(src, cursor)?),
    h3 = |src, cursor| LineChild::H3(H3::parse(src, cursor)?),
    keycode = |src, cursor| LineChild::Keycode(Keycode::parse(src, cursor)?),
    optionlink = |src, cursor| LineChild::Optionlink(Optionlink::parse(src, cursor)?),
    tag = |src, cursor| LineChild::Tag(Tag::parse(src, cursor)?),
    taglink = |src, cursor| LineChild::Taglink(Taglink::parse(src, cursor)?),
    url = |src, cursor| LineChild::Url(Url::parse(src, cursor)?),
    word = |src, cursor| LineChild::Word(Word::parse(src, cursor)?),
);

parse!(LineLi, line_li = @many(children = LineLiChild));

parse!(
    LineLiChild,
    codeblock = |src, cursor| LineLiChild::Codeblock(Codeblock::parse(src, cursor)?),
    line = |src, cursor| LineLiChild::Line(Line::parse(src, cursor)?),
);

parse!(Argument, argument = @single(text = Word));

parse!(
    Codeblock,
    codeblock = |src, cursor| {
        let mut language = None;
        let mut children = Vec::new();

        if cursor.goto_first_child() {
            let mut looking_for_language = true;
            loop {
                let node = cursor.node();
                if let Some(invalid_node) = InvalidNode::from_node_ref(&node) {
                    invalid_node.report();
                } else if node.is_named() {
                    // Language is optional and may appear first
                    if looking_for_language {
                        looking_for_language = false;

                        language = Language::parse(src, cursor).ok();

                        // If no language first, try a line
                        if language.is_none() {
                            children.push(Line::parse(src, cursor)?);
                        }
                    } else {
                        children.push(Line::parse(src, cursor)?);
                    }
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }

        Codeblock { language, children }
    }
);

parse!(Codespan, codespan = @single(text = Word));

parse!(ColumnHeading, column_heading = @many(name = HChild));

parse!(H1, h1 = @many(children = HChild));

parse!(H2, h2 = @many(children = HChild));

parse!(
    H3,
    h3 = |src, cursor| {
        let node = cursor.node();
        let mut name = None;
        let mut children = Vec::new();

        if cursor.goto_first_child() {
            let mut looking_for_name = true;
            loop {
                let node = cursor.node();
                if let Some(invalid_node) = InvalidNode::from_node_ref(&node) {
                    invalid_node.report();
                } else if node.is_named() {
                    if looking_for_name {
                        name = Some(UppercaseName::parse(src, cursor)?);
                        looking_for_name = false;
                    } else {
                        children.push(HChild::parse(src, cursor)?);
                    }
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }

        if name.is_none() {
            return Err(ParseError::MissingField {
                start: node.start_position(),
                name: "name",
                node_kind: node.kind().to_string(),
            });
        }

        H3 {
            name: name.unwrap(),
            children,
        }
    }
);

parse!(
    HChild,
    argument = |src, cursor| HChild::Argument(Argument::parse(src, cursor)?),
    codespan = |src, cursor| HChild::Codespan(Codespan::parse(src, cursor)?),
    keycode = |src, cursor| HChild::Keycode(Keycode::parse(src, cursor)?),
    optionlink = |src, cursor| HChild::Optionlink(Optionlink::parse(src, cursor)?),
    tag = |src, cursor| HChild::Tag(Tag::parse(src, cursor)?),
    taglink = |src, cursor| HChild::Taglink(Taglink::parse(src, cursor)?),
    url = |src, cursor| HChild::Url(Url::parse(src, cursor)?),
    word = |src, cursor| HChild::Word(Word::parse(src, cursor)?),
);

parse!(Optionlink, optionlink = @single(text = Word));

parse!(Tag, tag = @single(text = Word));

parse!(Taglink, taglink = @single(text = Word));

parse!(Url, url = @single(text = Word));

parse!(
    Keycode,
    keycode = |src, cursor| Keycode(node_text(src, cursor.node()))
);

parse!(
    Language,
    language = |src, cursor| Language(node_text(src, cursor.node()))
);

parse!(
    UppercaseName,
    uppercase_name = |src, cursor| UppercaseName(node_text(src, cursor.node()))
);

parse!(
    Word,
    word = |src, cursor| Word(node_text(src, cursor.node()))
);
