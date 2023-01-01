use std::io;

mod convert;
mod visit;

pub use convert::*;
pub use visit::*;

/// Represents the parser of vimdoc that leverages treesitter to construct a tree and then
/// translates the tree into an intermediate series of Rust structs representing vimdoc.
pub struct Parser {
    /// Content loaded into memory and being parsed.
    src: String,

    /// Tree representing parsed tree sitter of `src`.
    tree: tree_sitter::Tree,
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

    /// Returns a reference to the souce being parsed.
    pub fn src(&self) -> &str {
        &self.src
    }

    /// Returns a reference to the raw tree representing the source.
    pub fn tree(&self) -> &tree_sitter::Tree {
        &self.tree
    }

    /// Parses using the defined [`tree_sitter::Language`] into the type that implements
    /// [`FromParser`].
    pub fn parse<F>(&self) -> Result<F, <F as FromParser>::Err>
    where
        F: FromParser,
    {
        F::from_parser(self)
    }
}
