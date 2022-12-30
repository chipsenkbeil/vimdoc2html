use std::str::Utf8Error;
use tree_sitter::{Point, TreeCursor};

#[derive(Debug)]
pub enum FromCursorError {
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

impl std::fmt::Display for FromCursorError {
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

impl std::error::Error for FromCursorError {}

impl From<Utf8Error> for FromCursorError {
    fn from(x: Utf8Error) -> Self {
        Self::Utf8Error { err: x }
    }
}

macro_rules! concat_kinds {
    ($kind:ident, $($rest:ident),* $(,)?) => {
        concat!(stringify!($kind), $(" or ", stringify!($rest)),*)
    };
}

macro_rules! from_cursor {
    ($name:ident, $($kind:ident = |$src:ident, $cursor:ident| $body:expr),+ $(,)?) => {
        impl<'src, 'tree> $name<'src> {
            pub fn from_cursor(
                src: &'src str,
                cursor: &'tree mut TreeCursor,
            ) -> Result<$name<'src>, FromCursorError> {
                let node = cursor.node();

                match node.kind() {
                    $(
                        stringify!($kind) => {
                            fn _impl<'src, 'tree>(
                                $src: &'src str,
                                $cursor: &'tree mut TreeCursor,
                            ) -> Result<$name<'src>, FromCursorError> {
                                Ok($body)
                            }

                            _impl(src.as_ref(), cursor)
                        }
                    )+

                    _ => {
                        let expected = concat_kinds!($($kind,)+);
                        Err(FromCursorError::TypeError {
                            start: node.start_position(),
                            expected: expected.to_string(),
                            actual: node.kind().to_string(),
                        })
                    }
                }
            }
        }
    };
}

macro_rules! from_cursor_children {
    ($name:ident, $kind:ident, $children:ident) => {
        from_cursor_children!($name, $kind, children = $children);
    };
    ($name:ident, $kind:ident, $field:ident = $children:ident) => {
        from_cursor!(
            $name,
            $kind = |src, cursor| {
                let mut children = Vec::new();

                if cursor.goto_first_child() {
                    loop {
                        let node = cursor.node();
                        if node.is_named() {
                            let child = $children::from_cursor(src.as_ref(), cursor)?;
                            children.push(child);
                        }

                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                    cursor.goto_parent();
                }

                $name { $field: children }
            }
        );
    };
}

macro_rules! from_cursor_single_child {
    ($name:ident, $kind:ident, $child_field:ident = $child_struct:ident) => {
        from_cursor!(
            $name,
            $kind = |src, cursor| {
                let mut $child_field = None;
                let node = cursor.node();

                if cursor.goto_first_child() {
                    let mut cnt = 0;
                    loop {
                        let node = cursor.node();
                        if node.is_named() {
                            cnt += 1;
                            if cnt == 1 {
                                let child = $child_struct::from_cursor(src.as_ref(), cursor)?;
                                $child_field = Some(child);
                            }
                        }

                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }

                    cursor.goto_parent();

                    if cnt > 1 {
                        return Err(FromCursorError::TooManyChildren {
                            start: node.start_position(),
                            expected: 1,
                            actual: cnt,
                            node_kind: node.kind().to_string(),
                        });
                    }
                }

                if $child_field.is_none() {
                    return Err(FromCursorError::MissingField {
                        start: node.start_position(),
                        name: stringify!($child_field),
                        node_kind: node.kind().to_string(),
                    });
                }

                $name {
                    $child_field: $child_field.unwrap(),
                }
            }
        );
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelpFile<'a> {
    pub children: Vec<Block<'a>>,
}

from_cursor_children!(HelpFile, help_file, Block);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block<'a> {
    pub children: Vec<BlockChild<'a>>,
}

from_cursor_children!(Block, block, BlockChild);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockChild<'a> {
    Line(Line<'a>),
    LineLi(LineLi<'a>),
}

from_cursor!(
    BlockChild,
    line = |src, cursor| BlockChild::Line(Line::from_cursor(src, cursor)?),
    line_li = |src, cursor| BlockChild::LineLi(LineLi::from_cursor(src, cursor)?),
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line<'a> {
    pub children: Vec<LineChild<'a>>,
}

from_cursor_children!(Line, line, LineChild);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineChild<'a> {
    Argument(Argument<'a>),
    Codeblock(Codeblock<'a>),
    Codespan(Codespan<'a>),
    ColumnHeading(ColumnHeading<'a>),
    H1(H1<'a>),
    H2(H2<'a>),
    H3(H3<'a>),
    Keycode(Keycode<'a>),
    Optionlink(Optionlink<'a>),
    Tag(Tag<'a>),
    Taglink(Taglink<'a>),
    Url(Url<'a>),
    Word(Word<'a>),
}

from_cursor!(
    LineChild,
    argument = |src, cursor| LineChild::Argument(Argument::from_cursor(src, cursor)?),
    codeblock = |src, cursor| LineChild::Codeblock(Codeblock::from_cursor(src, cursor)?),
    codespan = |src, cursor| LineChild::Codespan(Codespan::from_cursor(src, cursor)?),
    column_heading =
        |src, cursor| LineChild::ColumnHeading(ColumnHeading::from_cursor(src, cursor)?),
    h1 = |src, cursor| LineChild::H1(H1::from_cursor(src, cursor)?),
    h2 = |src, cursor| LineChild::H2(H2::from_cursor(src, cursor)?),
    h3 = |src, cursor| LineChild::H3(H3::from_cursor(src, cursor)?),
    keycode = |src, cursor| LineChild::Keycode(Keycode::from_cursor(src, cursor)?),
    optionlink = |src, cursor| LineChild::Optionlink(Optionlink::from_cursor(src, cursor)?),
    tag = |src, cursor| LineChild::Tag(Tag::from_cursor(src, cursor)?),
    taglink = |src, cursor| LineChild::Taglink(Taglink::from_cursor(src, cursor)?),
    url = |src, cursor| LineChild::Url(Url::from_cursor(src, cursor)?),
    word = |src, cursor| LineChild::Word(Word::from_cursor(src, cursor)?),
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineLi<'a> {
    pub children: Vec<LineLiChild<'a>>,
}

from_cursor_children!(LineLi, line_li, LineLiChild);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineLiChild<'a> {
    Codeblock(Codeblock<'a>),
    Line(Line<'a>),
}

from_cursor!(
    LineLiChild,
    codeblock = |src, cursor| LineLiChild::Codeblock(Codeblock::from_cursor(src, cursor)?),
    line = |src, cursor| LineLiChild::Line(Line::from_cursor(src, cursor)?),
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument<'a> {
    text: Word<'a>,
}

from_cursor_single_child!(Argument, argument, text = Word);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codeblock<'a> {
    pub language: Option<Language<'a>>,
    pub children: Vec<Line<'a>>,
}

from_cursor!(
    Codeblock,
    codeblock = |src, cursor| {
        let mut language = None;
        let mut children = Vec::new();

        if cursor.goto_first_child() {
            let mut looking_for_language = true;
            loop {
                let node = cursor.node();
                if node.is_named() {
                    // Language is optional and may appear first
                    if looking_for_language {
                        looking_for_language = false;

                        language = Language::from_cursor(src, cursor).ok();

                        // If no language first, try a line
                        if language.is_none() {
                            children.push(Line::from_cursor(src, cursor)?);
                        }
                    } else {
                        children.push(Line::from_cursor(src, cursor)?);
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codespan<'a> {
    pub text: Word<'a>,
}

from_cursor_single_child!(Codespan, codespan, text = Word);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnHeading<'a> {
    pub name: Vec<HChild<'a>>,
}

from_cursor_children!(ColumnHeading, column_heading, name = HChild);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct H1<'a> {
    pub children: Vec<HChild<'a>>,
}

from_cursor_children!(H1, h1, HChild);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct H2<'a> {
    pub children: Vec<HChild<'a>>,
}

from_cursor_children!(H2, h2, HChild);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct H3<'a> {
    pub name: UppercaseName<'a>,
    pub children: Vec<HChild<'a>>,
}

from_cursor!(
    H3,
    h3 = |src, cursor| {
        let node = cursor.node();
        let mut name = None;
        let mut children = Vec::new();

        if cursor.goto_first_child() {
            let mut looking_for_name = true;
            loop {
                let node = cursor.node();
                if node.is_named() {
                    if looking_for_name {
                        name = Some(UppercaseName::from_cursor(src, cursor)?);
                        looking_for_name = false;
                    } else {
                        children.push(HChild::from_cursor(src, cursor)?);
                    }
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }

        if name.is_none() {
            return Err(FromCursorError::MissingField {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HChild<'a> {
    Argument(Argument<'a>),
    Codespan(Codespan<'a>),
    Keycode(Keycode<'a>),
    Optionlink(Optionlink<'a>),
    Tag(Tag<'a>),
    Taglink(Taglink<'a>),
    Url(Url<'a>),
    Word(Word<'a>),
}

from_cursor!(
    HChild,
    argument = |src, cursor| HChild::Argument(Argument::from_cursor(src, cursor)?),
    codespan = |src, cursor| HChild::Codespan(Codespan::from_cursor(src, cursor)?),
    keycode = |src, cursor| HChild::Keycode(Keycode::from_cursor(src, cursor)?),
    optionlink = |src, cursor| HChild::Optionlink(Optionlink::from_cursor(src, cursor)?),
    tag = |src, cursor| HChild::Tag(Tag::from_cursor(src, cursor)?),
    taglink = |src, cursor| HChild::Taglink(Taglink::from_cursor(src, cursor)?),
    url = |src, cursor| HChild::Url(Url::from_cursor(src, cursor)?),
    word = |src, cursor| HChild::Word(Word::from_cursor(src, cursor)?),
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Optionlink<'a> {
    pub text: Word<'a>,
}

from_cursor_single_child!(Optionlink, optionlink, text = Word);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag<'a> {
    pub text: Word<'a>,
}

from_cursor_single_child!(Tag, tag, text = Word);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Taglink<'a> {
    pub text: Word<'a>,
}

from_cursor_single_child!(Taglink, taglink, text = Word);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url<'a> {
    pub text: Word<'a>,
}

from_cursor_single_child!(Url, url, text = Word);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keycode<'a>(pub &'a str);

from_cursor!(
    Keycode,
    keycode = |src, cursor| Keycode(cursor.node().utf8_text(src.as_ref())?)
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Language<'a>(pub &'a str);

from_cursor!(
    Language,
    language = |src, cursor| Language(cursor.node().utf8_text(src.as_ref())?)
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UppercaseName<'a>(pub &'a str);

from_cursor!(
    UppercaseName,
    uppercase_name = |src, cursor| UppercaseName(cursor.node().utf8_text(src.as_ref())?)
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word<'a>(pub &'a str);

from_cursor!(
    Word,
    word = |src, cursor| Word(cursor.node().utf8_text(src.as_ref())?)
);
