use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::path::PathBuf;

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

    /// Paths to convert from vimdoc into html. If no paths are provided, will read vimdoc from
    /// stdin until EOF detected and then print out the html.
    paths: Vec<PathBuf>,
}

fn main() {
    let Args {
        extensions,
        recursive,
        paths,
    } = <Args as clap::Parser>::parse();
    let should_read_stdin = paths.is_empty();

    let mut parser = make_vimdoc_parser();

    // If we are reading stdin, then we block until we get all input, feed it into our parser, and
    // then print out the results
    if should_read_stdin {
        let tree = parse_into_tree(std::io::stdin(), &mut parser).unwrap();
        println!("{tree:#?}");
        return;
    }

    // Otherwise, we read in all of the paths and process sequentially.
    //
    // * For a file, we read it in as a byte
    let mut paths: VecDeque<PathBuf> = paths.into();
    while let Some(path) = paths.pop_front() {
        if path.is_file() {
            let outfile = path.with_extension("html");
            let tree = parse_into_tree(File::open(path).expect("Failed to open file"), &mut parser)
                .unwrap();
            std::fs::write(outfile, format!("{tree:#?}")).expect("Failed to write output");
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

fn parse_into_tree<R: io::Read>(
    mut reader: R,
    parser: &mut tree_sitter::Parser,
) -> io::Result<tree_sitter::Tree> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    parser
        .parse(buf, None)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to parse vimdoc"))
}
