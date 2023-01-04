mod debug;
mod html;

pub use debug::DebugString;
pub use html::HtmlString;

use crate::utils;
use crate::{Context, Joiner, NodeType, Parser, Visitor};

/// Parse a value from a [`Parser`].
pub trait FromParser: Sized {
    type Err;

    fn from_parser(parser: &Parser) -> Result<Self, Self::Err>;
}

pub trait VimdocTranslator {
    type Output;

    fn argument<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn block<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn code<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn codeblock<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn codespan<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn column_heading<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn h1<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn h2<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn h3<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn help_file<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn keycode<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn language<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn line<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn line_li<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn optionlink<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn tag<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn taglink<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn uppercase_name<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn url<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
    fn word<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output;
}

/// Options for the convert visitor.
#[derive(Clone, Debug)]
pub struct ConverterOpt<T> {
    pub joiner: T,
    pub old: bool,
}

/// State for the convert visitor.
#[derive(Debug, Default)]
pub struct ConverterState {
    pub language: Option<String>,
}

/// Used to convert into some other form by navigating a vimdoc tree.
pub struct Converter<T> {
    opt: ConverterOpt<T>,
    state: ConverterState,
}

impl<T> Converter<T> {
    pub fn new(opt: ConverterOpt<T>) -> Self {
        Self {
            opt,
            state: ConverterState::default(),
        }
    }
}

impl<T: Joiner<Output = String>> Visitor for Converter<T> {
    type Output = String;

    fn visit<'src, 'tree>(&mut self, ctx: &mut Context<'src, 'tree, '_>) -> Self::Output {
        let has_error = ctx.has_error();
        let text = if !ctx.has_children() || has_error {
            ctx.node_clean_text()
        } else {
            self.opt.joiner.join(self.visit_children_named(ctx))
        };
        let trimmed_text = text.trim_start();

        if let Some(node_type) = ctx.node_type() {
            match node_type {
                ///////////////////////////////////////////////////////////
                // NON-HTML GENERATION (PLAIN TEXT, ERROR HANDLING, ETC)
                ///////////////////////////////////////////////////////////
                NodeType::Block | NodeType::Code if utils::is_blank(&text) => String::new(),
                NodeType::ColumnHeading
                | NodeType::Codespan
                | NodeType::Keycode
                | NodeType::Tag
                    if has_error =>
                {
                    text
                }
                NodeType::H1 | NodeType::H2 | NodeType::H3 if utils::is_noise(&text) => {
                    String::new()
                }

                ///////////////////////////////////////////////////////////
                // HTML GENERATION
                ///////////////////////////////////////////////////////////
                NodeType::Argument => format!(r"<code>{text}</code>"),
                NodeType::Block if self.opt.old => {
                    format!(r#"<div class="old-help-para">{}</div>\n"#, text.trim_end())
                }
                NodeType::Block => format!(r#"<div class="help-para">\n{text}\n</div>\n"#),
                NodeType::Code => {
                    let text = utils::trim_indent(&text, /* tab=8space */ 8);
                    let trimmed = text.trim_end();
                    match self.state.language.take() {
                        Some(language) => {
                            format!(
                                r#"<pre><code class="language-{language}">{trimmed}</code></pre>"#
                            )
                        }
                        None => format!("<pre>{trimmed}</pre>"),
                    }
                }
                NodeType::Codeblock => text,
                NodeType::Codespan if self.opt.old => todo!(),
                NodeType::Codespan => format!("<code>{trimmed_text}</code>"),
                NodeType::ColumnHeading => {
                    format!(r#"<div class="help-column_heading">{text}</div>"#)
                }
                NodeType::H1 => todo!(),
                NodeType::H2 => todo!(),
                NodeType::H3 => todo!(),
                NodeType::HelpFile => text,
                NodeType::Keycode => format!("<code>{trimmed_text}</code>"),
                NodeType::Language => {
                    self.state.language = Some(ctx.node_raw_text().to_string());
                    String::new()
                }
                NodeType::Line => todo!(),
                NodeType::LineLi => todo!(),
                NodeType::Optionlink => todo!(),
                NodeType::Tag => todo!(),
                NodeType::Taglink => todo!(),
                NodeType::UppercaseName => text,
                NodeType::Url => {
                    let (href, remaining) = utils::fix_url(trimmed_text);
                    format!(r#"<a href="{href}">{href}</a>{remaining}"#)
                }
                NodeType::Word => text,
            }
        } else if has_error && utils::ignore_parse_error(trimmed_text) {
            text
        } else if has_error {
            let text = utils::truncate_str(&text, 10);
            format!(r#"{{ERROR: {text}}}"#)
        } else {
            String::new()
        }
    }
}
