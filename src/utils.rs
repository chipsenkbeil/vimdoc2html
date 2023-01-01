use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

static EXCLUDE_INVALID: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    vec![
        ("'previewpopup'", "quickref.txt"),
        ("'pvp'", "quickref.txt"),
        ("'string'", "eval.txt"),
        ("Query", "treesitter.txt"),
        ("eq?", "treesitter.txt"),
        ("lsp-request", "lsp.txt"),
        ("matchit", "vim_diff.txt"),
        ("matchit.txt", "help.txt"),
        ("set!", "treesitter.txt"),
        ("v:_null_blob", "builtin.txt"),
        ("v:_null_dict", "builtin.txt"),
        ("v:_null_list", "builtin.txt"),
        ("v:_null_string", "builtin.txt"),
        ("vim.lsp.buf_request()", "lsp.txt"),
        ("vim.lsp.util.get_progress_messages()", "lsp.txt"),
        ("vim.treesitter.start()", "treesitter.txt"),
    ]
    .into_iter()
    .collect()
});

pub fn truncate_str(s: &str, cnt: usize) -> &str {
    &s[..floor_char_boundary(s, cnt)]
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

/// Ignore parse errors for unclosed tag. This is common in vimdocs and is treated as plaintext by
/// :help.
///
/// Port of https://github.com/neovim/neovim/blob/99cf111289bfcd14981255e805da43bac5139141/scripts/gen_help_html.lua#L299
#[inline]
pub fn ignore_parse_error(s: &str) -> bool {
    matches!(s.chars().next(), Some('`' | '\'' | '|' | '*'))
}

/// Returns true if the given invalid tagname is a false positive.
///
/// Port of https://github.com/neovim/neovim/blob/99cf111289bfcd14981255e805da43bac5139141/scripts/gen_help_html.lua#L289
pub fn ignore_invalid(s: &str) -> bool {
    EXCLUDE_INVALID.contains_key(s) || s.contains("===") || s.contains("---")
}

/// Returns true if str is entirely comprised of tabs and/or spaces.
pub fn is_blank(s: &str) -> bool {
    s.chars().all(|c| c == '\t' || c == ' ')
}

/// Checks if a given line is a "noise" line that doesn't look good in HTML form.
///
/// Port of https://github.com/neovim/neovim/blob/99cf111289bfcd14981255e805da43bac5139141/scripts/gen_help_html.lua#L169
pub fn is_noise(s: &str) -> bool {
    static TOC_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"Type .*gO.* to see the table of contents"#).unwrap());

    // Title line of traditional :help pages.
    // Example: "NVIM REFERENCE MANUAL    by ..."
    static TITLE_LINE_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"^\s*N?VIM[ \t]*REFERENCE[ \t]*MANUAL"#).unwrap());

    // First line of traditional :help pages.
    // Example: "*api.txt*    Nvim"
    static HELP_FIRST_LINE_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"\s*\*?[a-zA-Z]+\.txt\*?\s+N?[vV]im\s*$"#).unwrap());

    // modeline
    // Example: "vim:tw=78:ts=8:sw=4:sts=4:et:ft=help:norl:"
    static MODELINE_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"^\s*vim?:.*ft=help|^\s*vim?:.*filetype=help|[*>]local\-additions[*<]"#)
            .unwrap()
    });

    // First line is always noise.
    // (noise_lines ~= nil and vim.tbl_count(noise_lines) == 0)
    TOC_RE.is_match(s)
        || TITLE_LINE_RE.is_match(s)
        || HELP_FIRST_LINE_RE.is_match(s)
        || MODELINE_RE.is_match(s)
}

// Port of Lua
// https://github.com/neovim/neovim/blob/6ba34e21fee2a81677e8261dfeaf24c8cd320500/scripts/gen_help_html.lua#L155
pub fn fix_url(url: &str) -> (&str, &str) {
    let mut remaining_len = 0;
    if url.ends_with('.') {
        remaining_len += 1;
    }
    if url[..url.len() - remaining_len].ends_with(')') {
        remaining_len += 1;
    }
    url.split_at(url.len() - remaining_len)
}

/// Removes leading whitespace from each line to match furthest-left line. Will convert tabs to
/// `tab_to_space_cnt` spaces.
pub fn trim_indent(s: &str, tab_to_space_cnt: usize) -> String {
    let expanded = s.replace('\t', &" ".repeat(tab_to_space_cnt));
    let remove_cnt = expanded
        .lines()
        .map(|line| line.chars().take_while(|c| *c == ' ').count())
        .min()
        .unwrap_or(0);
    expanded
        .lines()
        .map(|line| line.chars().skip(remove_cnt).collect())
        .collect::<Vec<String>>()
        .join("\n")
}
