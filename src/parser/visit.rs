use std::fmt;
use std::str::FromStr;

#[macro_export]
macro_rules! visitor {
    ($($keyword:ident)* |$this:ident, $ctx:ident| -> $output:ty $body:block) => {{
        struct __VisitorImpl;

        impl Visitor for __VisitorImpl {
            type Output = $output;

            fn visit<'src, 'tree>(&mut self, $ctx: &mut Context<'src, 'tree, '_>) -> Self::Output {
                let $this = self;
                $body
            }
        }

        __VisitorImpl
    }};
    ($($keyword:ident)* |$ctx:ident| $body:block) => {{
        visitor!($($keyword)* |$ctx| -> () $body)
    }};
}

/// Interface providing the ability to join multiple outputs together.
pub trait Joiner {
    type Output;

    fn join(&self, outputs: Vec<Self::Output>) -> Self::Output;
}

/// Implementation of [`Joiner`] that uses a separator to join multiple [`String`].
pub struct StringJoiner<'a> {
    sep: &'a str,
}

impl<'a> StringJoiner<'a> {
    pub const fn new(sep: &'a str) -> Self {
        Self { sep }
    }
}

impl<'a> Joiner for StringJoiner<'a> {
    type Output = String;

    fn join(&self, outputs: Vec<Self::Output>) -> Self::Output {
        outputs.join(self.sep)
    }
}

/// Instance of [`StringJoiner`] whose separator is `\n`.
pub const NEWLINE_STRING_JOINER: StringJoiner<'static> = StringJoiner::new("\n");

/// Instance of [`StringJoiner`] whose separator is ` `.
pub const SPACE_STRING_JOINER: StringJoiner<'static> = StringJoiner::new(" ");

/// Interface that handles visiting different tree nodes in order to generate some output.
pub trait Visitor {
    type Output;

    /// Visits a single node tied to the [`Context`].
    fn visit<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;

    /// Visits all named nodes starting with the root defined in the given [`Context]. Nodes are
    /// visited in pre-order, meaning parent followed by all children from left-to-right.
    fn visit_all_named<'src, 'tree, J: Joiner<Output = Self::Output>>(
        &mut self,
        ctx: &mut Context<'src, 'tree, '_>,
        joiner: &J,
    ) -> Self::Output {
        self.visit_all(ctx, joiner, /* unnamed */ false)
    }

    /// Visits all nodes starting with the root defined in the given [`Context]. Nodes are visited
    /// in pre-order, meaning parent followed by all children from left-to-right.
    ///
    /// If `unnamed` is true, then nodes that are unnamed will also be visited.
    fn visit_all<'src, 'tree, J: Joiner<Output = Self::Output>>(
        &mut self,
        ctx: &mut Context<'src, 'tree, '_>,
        joiner: &J,
        unnamed: bool,
    ) -> Self::Output {
        // Vec<Vec<Self::Output>> where top-level index is our depth
        let mut outputs: Vec<Vec<Self::Output>> = vec![Vec::new()];

        loop {
            if ctx.node().is_named() || unnamed {
                outputs.last_mut().unwrap().push(self.visit(ctx));
            }

            if ctx.cursor.goto_first_child() {
                outputs.push(Vec::new());
                continue;
            }

            if ctx.cursor.goto_next_sibling() {
                continue;
            }

            // Recurse back up to find next sibling from a parent
            loop {
                // If no parent left, this means we've gone through the entire tree
                // and have reached the root node
                if !ctx.cursor.goto_parent() {
                    return joiner.join(outputs.into_iter().next().unwrap());
                }

                // Went up a level, so join everything at the depth we just left
                // and add it to our current depth
                if let Some(output) = outputs.pop().map(|x| joiner.join(x)) {
                    match outputs.last_mut() {
                        Some(x) => x.push(output),
                        None => return output,
                    }
                }

                // Otherwise, attempt to go to the next sibling and, if successful,
                // we are done with this retracing loop
                if ctx.cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn visit_children_named<'src, 'tree, J: Joiner<Output = Self::Output>>(
        &mut self,
        ctx: &mut Context<'src, 'tree, '_>,
        joiner: &J,
    ) -> Self::Output {
        self.visit_children(ctx, joiner, /* unnamed */ false)
    }

    /// Visits all immediate children nodes from the root defined in the [`Context]. The root node
    /// itself is NOT visited.
    ///
    /// If `unnamed` is true, then nodes that are unnamed will also be visited.
    fn visit_children<'src, 'tree, J: Joiner<Output = Self::Output>>(
        &mut self,
        ctx: &mut Context<'src, 'tree, '_>,
        joiner: &J,
        unnamed: bool,
    ) -> Self::Output {
        let mut outputs = Vec::new();

        if !ctx.cursor.goto_first_child() {
            return joiner.join(outputs);
        }

        loop {
            if ctx.node().is_named() || unnamed {
                outputs.push(self.visit(ctx));
            }

            if !ctx.cursor.goto_next_sibling() {
                return joiner.join(outputs);
            }
        }
    }
}

/// Maintains context throughout visiting nodes in a tree.
pub struct Context<'src, 'tree, 'cursor> {
    pub(super) src: &'src str,
    pub(super) cursor: &'cursor mut tree_sitter::TreeCursor<'tree>,
}

impl<'src, 'tree> Context<'src, 'tree, '_> {
    /// Returns the source tied to the tree being traversed.
    #[inline]
    pub fn src(&self) -> &'src str {
        self.src
    }

    /// Returns the node being visited.
    #[inline]
    pub fn node(&self) -> tree_sitter::Node {
        self.cursor.node()
    }

    /// Returns the type of node being visited.
    #[inline]
    pub fn node_type(&self) -> Option<NodeType> {
        self.node().node_type()
    }

    /// Returns the raw text represented by the node being visited.
    #[inline]
    pub fn node_raw_text(&self) -> &'src str {
        self.node().utf8_text(self.src.as_bytes()).unwrap()
    }

    /// Returns cleaned text represented by the node being visited.
    #[inline]
    pub fn node_clean_text(&self) -> String {
        self.node_raw_text()
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    /// Returns true if the node being visited is an error or has errors in children nodes or
    /// deeper. An error can be represented as a node with ERROR, MISSING, or other internal node
    /// types.
    #[inline]
    pub fn has_error(&self) -> bool {
        self.node().has_error()
    }

    /// Returns true if the node being visited has children.
    #[inline]
    pub fn has_children(&self) -> bool {
        self.node().named_child_count() > 0
    }
}

/// Interface providing additional methods for [`tree_sitter::Node`].
pub trait NodeExt {
    /// Calculates and returns the depth of the node in the tree where 0 is the root of the tree.
    fn depth(&self) -> usize;

    /// Returns vimdoc type associated with the node, or none if the type is unknown or an error.
    fn node_type(&self) -> Option<NodeType>;

    /// Returns vimdoc type associated with the node's parent.
    fn parent_node_type(&self) -> Option<NodeType>;

    /// Returns vimdoc type associated with the node's previous sibling.
    fn prev_sibling_node_type(&self) -> Option<NodeType>;

    /// Returns vimdoc type associated with the node's next sibling.
    fn next_sibling_node_type(&self) -> Option<NodeType>;
}

impl NodeExt for tree_sitter::Node<'_> {
    fn depth(&self) -> usize {
        let mut depth = 0;

        let mut parent = self.parent();
        while let Some(node) = parent {
            depth += 1;
            parent = node.parent();
        }

        depth
    }

    fn node_type(&self) -> Option<NodeType> {
        self.kind().parse().ok()
    }

    fn parent_node_type(&self) -> Option<NodeType> {
        self.parent().and_then(|node| node.node_type())
    }

    fn prev_sibling_node_type(&self) -> Option<NodeType> {
        self.prev_named_sibling().and_then(|node| node.node_type())
    }

    fn next_sibling_node_type(&self) -> Option<NodeType> {
        self.next_named_sibling().and_then(|node| node.node_type())
    }
}

/// Represents types of nodes that can be encountered when navigating a vimdoc.
#[non_exhaustive]
pub enum NodeType {
    Argument,
    Block,
    Code,
    Codeblock,
    Codespan,
    ColumnHeading,
    H1,
    H2,
    H3,
    HelpFile,
    Keycode,
    Language,
    Line,
    LineLi,
    Optionlink,
    Tag,
    Taglink,
    UppercaseName,
    Url,
    Word,
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Argument => write!(f, "argument"),
            Self::Block => write!(f, "block"),
            Self::Code => write!(f, "code"),
            Self::Codeblock => write!(f, "codeblock"),
            Self::Codespan => write!(f, "codespan"),
            Self::ColumnHeading => write!(f, "column_heading"),
            Self::H1 => write!(f, "h1"),
            Self::H2 => write!(f, "h2"),
            Self::H3 => write!(f, "h3"),
            Self::HelpFile => write!(f, "help_file"),
            Self::Keycode => write!(f, "keycode"),
            Self::Language => write!(f, "language"),
            Self::Line => write!(f, "line"),
            Self::LineLi => write!(f, "line_li"),
            Self::Optionlink => write!(f, "optionlink"),
            Self::Tag => write!(f, "tag"),
            Self::Taglink => write!(f, "taglink"),
            Self::UppercaseName => write!(f, "uppercase_name"),
            Self::Url => write!(f, "url"),
            Self::Word => write!(f, "word"),
        }
    }
}

impl FromStr for NodeType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "argument" => Ok(Self::Argument),
            "block" => Ok(Self::Block),
            "code" => Ok(Self::Code),
            "codeblock" => Ok(Self::Codeblock),
            "codespan" => Ok(Self::Codespan),
            "column_heading" => Ok(Self::ColumnHeading),
            "h1" => Ok(Self::H1),
            "h2" => Ok(Self::H2),
            "h3" => Ok(Self::H3),
            "help_file" => Ok(Self::HelpFile),
            "keycode" => Ok(Self::Keycode),
            "language" => Ok(Self::Language),
            "line" => Ok(Self::Line),
            "line_li" => Ok(Self::LineLi),
            "optionlink" => Ok(Self::Optionlink),
            "tag" => Ok(Self::Tag),
            "taglink" => Ok(Self::Taglink),
            "uppercase_name" => Ok(Self::UppercaseName),
            "url" => Ok(Self::Url),
            "word" => Ok(Self::Word),
            _ => Err(()),
        }
    }
}
