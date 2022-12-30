use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::path::PathBuf;

mod types;

/// Convert vimdoc into html.
#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File extensions to look for when converting a directory of vimdoc.
    #[arg(short, long, default_values_t = vec![String::from("txt")])]
    extensions: Vec<String>,

    /// If specified, will recursively look through directories for vimdoc files.
    #[arg(short, long)]
    recursive: bool,

    /// If specified, will write out a debug string instead of HTML.
    #[arg(long)]
    debug_output: bool,

    /// Paths to convert from vimdoc into html. If no paths are provided, will read vimdoc from
    /// stdin until EOF detected and then print out the html.
    paths: Vec<PathBuf>,
}

fn main() {
    let Args {
        extensions,
        recursive,
        debug_output,
        paths,
    } = <Args as clap::Parser>::parse();
    let should_read_stdin = paths.is_empty();

    let mut parser = make_vimdoc_parser();

    // If we are reading stdin, then we block until we get all input, feed it into our parser, and
    // then print out the results
    if should_read_stdin {
        let (src, tree) = parse_into_src_and_tree(std::io::stdin(), &mut parser).unwrap();
        let out = if debug_output {
            tree_into_debug_string(tree)
        } else {
            into_html_string(&src, tree)
        };
        println!("{out}");
        return;
    }

    // Otherwise, we read in all of the paths and process sequentially.
    //
    // * For a file, we read it in as a byte
    let mut paths: VecDeque<PathBuf> = paths.into();
    while let Some(path) = paths.pop_front() {
        if path.is_file() {
            let outfile = path.with_extension("html");
            let (src, tree) = parse_into_src_and_tree(
                File::open(path).expect("Failed to open file"),
                &mut parser,
            )
            .unwrap();
            let out = if debug_output {
                tree_into_debug_string(tree)
            } else {
                into_html_string(&src, tree)
            };
            std::fs::write(outfile, out).expect("Failed to write output");
        } else if path.is_dir() {
            for entry in std::fs::read_dir(path).expect("Failed to read directory") {
                let entry = entry.expect("Failed to read directory entry");
                let file_type = entry
                    .file_type()
                    .expect("Failed to read directory entry file type");
                let path = entry.path();
                let ext = path.extension().unwrap_or_else(|| OsStr::new(""));

                // Queue up the inner path if it is a file with a matching extension or
                // a directory when we have the recursive flag set
                if (file_type.is_file() && extensions.iter().any(|x| x.as_str() == ext))
                    || (file_type.is_dir() && recursive)
                {
                    paths.push_back(path);
                }
            }
        }
    }
}

fn make_vimdoc_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_vimdoc::language();
    parser.set_language(language).unwrap();
    parser
}

fn parse_into_src_and_tree<R: io::Read>(
    reader: R,
    parser: &mut tree_sitter::Parser,
) -> io::Result<(String, tree_sitter::Tree)> {
    let buf = std::io::read_to_string(reader)?;
    let tree = parser
        .parse(&buf, None)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to parse vimdoc"))?;
    Ok((buf, tree))
}

/// Converts [`tree_sitter::Tree`] into a debug [`String`].
fn tree_into_debug_string(tree: tree_sitter::Tree) -> String {
    let mut output = String::new();

    fn parent_cnt(node: &tree_sitter::Node) -> usize {
        match node.parent() {
            Some(node) => 1 + parent_cnt(&node),
            None => 0,
        }
    }

    for node in tree_sitter_traversal::traverse_tree(&tree, tree_sitter_traversal::Order::Pre) {
        if node.is_named() {
            let depth = parent_cnt(&node);

            output.push_str(&format!(
                "{}Kind: {:?} [Row:{}, Col:{}] - [Row:{}, Col:{}]\n",
                " ".repeat(depth * 4),
                node.kind(),
                node.start_position().row,
                node.start_position().column,
                node.end_position().row,
                node.end_position().column,
            ));
        }
    }

    output
}

/// Converts a `src` text and [`tree_sitter::Tree`] into an HTML [`String`].
fn into_html_string(src: &str, tree: tree_sitter::Tree) -> String {
    format!(
        "{:#?}",
        types::HelpFile::from_cursor(src, &mut tree.walk()).unwrap()
    )
}
