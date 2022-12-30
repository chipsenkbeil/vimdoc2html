#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelpFile<'a> {
    pub children: Vec<Block<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block<'a> {
    pub children: Vec<BlockChild<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockChild<'a> {
    Line(Line<'a>),
    LineLi(LineLi<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line<'a> {
    pub children: Vec<LineChild<'a>>,
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineLi<'a> {
    pub children: Vec<LineLiChild<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineLiChild<'a> {
    Codeblock(Codeblock<'a>),
    Line(Line<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument<'a> {
    pub text: Word<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codeblock<'a> {
    pub language: Option<Language<'a>>,
    pub children: Vec<Line<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codespan<'a> {
    pub text: Word<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnHeading<'a> {
    pub name: Vec<HChild<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct H1<'a> {
    pub children: Vec<HChild<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct H2<'a> {
    pub children: Vec<HChild<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct H3<'a> {
    pub name: UppercaseName<'a>,
    pub children: Vec<HChild<'a>>,
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Optionlink<'a> {
    pub text: Word<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag<'a> {
    pub text: Word<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Taglink<'a> {
    pub text: Word<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url<'a> {
    pub text: Word<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keycode<'a>(pub &'a str);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Language<'a>(pub &'a str);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UppercaseName<'a>(pub &'a str);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word<'a>(pub &'a str);
