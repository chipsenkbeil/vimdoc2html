use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs::File;
use std::path::PathBuf;

mod parser;
mod types;

use parser::*;

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

    /// If specified, will not print anything to stdout.
    #[arg(short, long)]
    quiet: bool,

    /// Paths to convert from vimdoc into html. If no paths are provided, will read vimdoc from
    /// stdin until EOF detected and then print out the html.
    paths: Vec<PathBuf>,
}

fn main() {
    let Args {
        extensions,
        recursive,
        debug_output,
        quiet,
        paths,
    } = <Args as clap::Parser>::parse();
    let should_read_stdin = paths.is_empty();

    // If we are reading stdin, then we block until we get all input, feed it into our parser, and
    // then print out the results
    if should_read_stdin {
        let parser = Parser::load_vimdoc(std::io::stdin()).expect("Failed to load parser");
        let out = if debug_output {
            parser.to_tree_debug_string()
        } else {
            format!(
                "{:#?}",
                parser.parse().expect("Failed to parse into vimdoc")
            )
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
            if !quiet {
                println!("Converting {path:?} into {outfile:?}");
            }

            let parser = Parser::load_vimdoc(File::open(path).expect("Failed to open file"))
                .expect("Failed to load parser");
            let out = if debug_output {
                parser.to_tree_debug_string()
            } else {
                format!(
                    "{:#?}",
                    parser.parse().expect("Failed to parse into vimdoc")
                )
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
