use std::str::Utf8Error;
use tree_sitter::{Node, TreeCursor};

#[derive(Debug)]
pub enum FromCursorError<'a> {
    MissingField {
        name: &'a str,
        node: Node<'a>,
    },
    TooManyChildren {
        expected: usize,
        actual: usize,
        node: Node<'a>,
    },
    TypeError {
        expected: &'a str,
        actual: Node<'a>,
    },
    Utf8Error(Utf8Error),
}

impl std::fmt::Display for FromCursorError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField { name, node } => {
                write!(f, "[{}] Missing {name}", node.kind())
            }
            Self::TooManyChildren {
                expected,
                actual,
                node,
            } => {
                write!(
                    f,
                    "[{}] Expected {expected} named children, but had {actual}",
                    node.kind()
                )
            }
            Self::TypeError { expected, actual } => {
                write!(f, "Expected {expected}, but was actually {}", actual.kind())
            }
            Self::Utf8Error(x) => write!(f, "{x}"),
        }
    }
}

impl std::error::Error for FromCursorError<'_> {}

impl From<Utf8Error> for FromCursorError<'static> {
    fn from(x: Utf8Error) -> Self {
        Self::Utf8Error(x)
    }
}

macro_rules! concat_kinds {
    ($kind:ident, $($rest:ident),* $(,)?) => {
        concat!(stringify!($kind), $(" or ", stringify!($rest)),*)
    };
}

macro_rules! from_cursor {
    ($name:ident, $($kind:ident = |$src:ident, $cursor:ident| $body:expr),+ $(,)?) => {
        impl<'a> $name<'a> {
            pub fn from_cursor(
                src: impl AsRef<[u8]>,
                cursor: &'a mut TreeCursor,
            ) -> Result<$name<'a>, FromCursorError<'a>> {
                let node = cursor.node();

                match node.kind() {
                    $(
                        stringify!($kind) => {
                            fn _impl<'a>(
                                $src: impl AsRef<[u8]>,
                                $cursor: &'a mut TreeCursor,
                            ) -> Result<$name<'a>, FromCursorError<'a>> {
                                Ok($body)
                            }

                            let result = _impl(src.as_ref(), cursor);
                            cursor.reset(node);
                            result
                        }
                    )+

                    x => {
                        cursor.reset(node);
                        let expected = concat_kinds!($($kind,)+);
                        Err(FromCursorError::TypeError {
                            expected,
                            actual: node,
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
                    let mut node = cursor.node();
                    loop {
                        if !node.is_named() {
                            continue;
                        }

                        children.push($children::from_cursor(src.as_ref(), cursor)?);

                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
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
                    let mut node = cursor.node();
                    loop {
                        if !node.is_named() {
                            continue;
                        }

                        cnt += 1;
                        if cnt == 1 {
                            $child_field = Some($child_struct::from_cursor(src.as_ref(), cursor)?);
                        }

                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }

                    if cnt > 1 {
                        return Err(FromCursorError::TooManyChildren {
                            expected: 1,
                            actual: cnt,
                            node,
                        });
                    }
                }

                if $child_field.is_none() {
                    return Err(FromCursorError::MissingField {
                        name: stringify!($child_field),
                        node,
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
pub struct Code<'a> {
    pub children: Vec<Line<'a>>,
}

from_cursor_children!(Code, code, Line);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codeblock<'a> {
    pub children: Vec<CodeblockChild<'a>>,
}

from_cursor_children!(Codeblock, codeblock, CodeblockChild);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeblockChild<'a> {
    Code(Code<'a>),
    Language(Language<'a>),
}

from_cursor!(
    CodeblockChild,
    code = |src, cursor| CodeblockChild::Code(Code::from_cursor(src, cursor)?),
    language = |src, cursor| CodeblockChild::Language(Language::from_cursor(src, cursor)?),
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
            let mut node = cursor.node();
            loop {
                if !node.is_named() {
                    continue;
                }

                if looking_for_name {
                    name = Some(UppercaseName::from_cursor(src.as_ref(), cursor)?);
                } else {
                    children.push(HChild::from_cursor(src.as_ref(), cursor)?);
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        if name.is_none() {
            return Err(FromCursorError::MissingField { name: "name", node });
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
