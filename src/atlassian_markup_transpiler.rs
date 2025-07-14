use std::collections::HashMap;

use chumsky::{prelude::*, text::digits};
use lsp_types::MarkupKind;
use regex::RegexBuilder;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AdmotionKind {
    Info,
    Tip,
    Warning,
    Note,
}

impl AdmotionKind {
    fn aatlassian_markup_keyword(&self) -> &'static str {
        match self {
            AdmotionKind::Info => "info",
            AdmotionKind::Tip => "tip",
            AdmotionKind::Warning => "warning",
            AdmotionKind::Note => "note",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MarkUpNode<'a> {
    PlainText(&'a str),
    Heading1(&'a str),
    Heading2(&'a str),
    Heading3(&'a str),
    Heading4(&'a str),
    Heading5(&'a str),
    Heading6(&'a str),
    CodeBlock {
        language: Option<&'a str>,
        content: &'a str,
    },
    Admotion {
        kind: AdmotionKind,
        title: Option<&'a str>,
        show_icon: bool,
        content: &'a str,
    },
}

impl<'a> MarkUpNode<'a> {
    pub fn to_markdown_string(&self) -> String {
        match self {
            MarkUpNode::PlainText(content) => format!("{}\n", content.to_owned()),
            MarkUpNode::Heading1(content) => format!("# {}\n", content),
            MarkUpNode::Heading2(content) => format!("## {}\n", content),
            MarkUpNode::Heading3(content) => format!("### {}\n", content),
            MarkUpNode::Heading4(content) => format!("#### {}\n", content),
            MarkUpNode::Heading5(content) => format!("##### {}\n", content),
            MarkUpNode::Heading6(content) => format!("###### {}\n", content),
            MarkUpNode::CodeBlock { language, content } => {
                format!("```{}\n{}\n```", language.unwrap_or(""), content)
            }
            MarkUpNode::Admotion {
                kind,
                title,
                show_icon,
                content,
            } => todo!("Output admition markdown"),
        }
    }

    pub fn push_content_onto_string(&self, target: &mut String) {
        match self {
            MarkUpNode::PlainText(content) => target.push_str(content),
            MarkUpNode::Heading1(content) => {
                target.push_str("# ");
                target.push_str(content);
            }
            MarkUpNode::Heading2(content) => {
                target.push_str("## ");
                target.push_str(content);
            }
            MarkUpNode::Heading3(content) => {
                target.push_str("### ");
                target.push_str(content);
            }
            MarkUpNode::Heading4(content) => {
                target.push_str("#### ");
                target.push_str(content);
            }
            MarkUpNode::Heading5(content) => {
                target.push_str("##### ");
                target.push_str(content);
            }
            MarkUpNode::Heading6(content) => {
                target.push_str("###### ");
                target.push_str(content);
            }
            MarkUpNode::CodeBlock { language, content } => {
                target.push_str("```");
                if let Some(lang) = language {
                    target.push_str(lang);
                }
                target.push_str("\n");
                target.push_str(content);
                target.push_str("```");
            }
            MarkUpNode::Admotion {
                kind,
                title,
                show_icon: _,
                content,
            } => {
                match kind {
                    AdmotionKind::Info => target.push_str("> [!INFO]"),
                    AdmotionKind::Tip => target.push_str("> [!TIP]"),
                    AdmotionKind::Warning => target.push_str("> [!WARNING]"),
                    AdmotionKind::Note => target.push_str("> [!NOTE]"),
                };
                if let Some(title) = title {
                    target.push_str("**");
                    target.push_str(&title);
                    target.push_str("**");
                }
                target.push_str(&content);
            }
        }
        target.push_str("\n");
    }
}

fn heading_ast_node_from_count<'a>(count: u32) -> impl Fn(&'a str) -> MarkUpNode<'a> {
    match count {
        1 => MarkUpNode::Heading1,
        2 => MarkUpNode::Heading2,
        3 => MarkUpNode::Heading3,
        4 => MarkUpNode::Heading4,
        5 => MarkUpNode::Heading5,
        6 => MarkUpNode::Heading6,
        _ => panic!("Illegal"),
    }
}

fn build_atlassian_markup_heading_parser<'a>() -> impl Parser<'a, &'a str, MarkUpNode<'a>> {
    let any_until_end_of_line = none_of("\n").repeated().to_slice().then_ignore(just("\n"));
    let digit_parser = one_of("123456").map(|digit_char: char| digit_char.to_digit(10).unwrap());
    let inline_whitespace = one_of(" \t").repeated();
    just("h")
        .ignore_then(digit_parser)
        .then_ignore(just("."))
        .then_ignore(inline_whitespace)
        .map(heading_ast_node_from_count)
        .then(any_until_end_of_line)
        .map(|(heading_ast_fn, heading_content)| heading_ast_fn(heading_content))
}

enum CodeBlockOption<'a> {
    Title(&'a str),
    LineNumbers(bool),
    Language(&'a str),
    FirstLine(u64),
    Collapse(bool),
}

fn build_code_block_parser<'a>() -> impl Parser<'a, &'a str, MarkUpNode<'a>> {
    let bool_parser = just("true")
        .or(just("false"))
        .from_str::<bool>()
        .unwrapped();
    // let nubmer_parser = any().filter(|c: &char| c.is_ascii_digit());
    let arguments_parser = just(":").ignore_then(
        choice((
            just("title=")
                .ignore_then(none_of("|").repeated().to_slice())
                .map(|v| CodeBlockOption::Title(v)),
            just("linenumbers=")
                .ignore_then(bool_parser)
                .map(|v| CodeBlockOption::LineNumbers(v)),
            just("language=")
                .ignore_then(text::ident())
                .map(|v| CodeBlockOption::Language(v)),
            just("firstline=")
                .ignore_then(digits(10).to_slice().from_str::<u64>().unwrapped())
                .map(|v| CodeBlockOption::FirstLine(v)),
            just("collapse=")
                .ignore_then(bool_parser)
                .map(|v| CodeBlockOption::Collapse(v)),
        ))
        .boxed()
        .separated_by(just("|"))
        .collect::<Vec<CodeBlockOption>>(),
    );

    let code_body = just("}")
        .padded()
        .ignore_then(any().and_is(just("{code}").not()).repeated().to_slice())
        .then_ignore(just("{code}").padded());

    just("{code")
        .ignore_then(arguments_parser.or_not())
        .then(code_body)
        .map(|(opts, inp)| MarkUpNode::CodeBlock {
            language: opts
                .map(|vs| {
                    vs.iter().find_map(|f| match *f {
                        CodeBlockOption::Language(lang) => Some(lang),
                        _ => None,
                    })
                })
                .flatten(),
            content: inp,
        })
}

enum AdmotionOption<'a> {
    Title(&'a str),
    ShowIcon(bool),
}

fn build_admotion_parser<'a>(
    admotion_kind: AdmotionKind,
) -> impl Parser<'a, &'a str, MarkUpNode<'a>, extra::Err<Rich<'a, char>>> {
    let title_parser = just("title=")
        .ignore_then(none_of("|}").repeated().to_slice())
        .map(AdmotionOption::Title);

    let show_icon_parser = just("show_icon=")
        .ignore_then(just("true").to(true).or(just("false").to(false)))
        .map(AdmotionOption::ShowIcon);

    let ops_parser = just(":").ignore_then(
        title_parser
            .or(show_icon_parser)
            .separated_by(just("|"))
            .collect::<Vec<AdmotionOption<'a>>>(),
    );

    let tag_prefix = just("{").then(just(admotion_kind.aatlassian_markup_keyword()));
    let start_tag = tag_prefix
        .ignore_then(ops_parser.or_not())
        .then_ignore(just("}"))
        .boxed();
    let end_tag = tag_prefix.then(just("}"));
    let code_content = any().and_is(end_tag.not()).repeated().to_slice();

    start_tag
        .padded()
        .then(code_content)
        .then_ignore(end_tag)
        .map(move |(ops, content)| MarkUpNode::Admotion {
            kind: admotion_kind,
            title: ops
                .as_ref()
                .map(|ops_vs| {
                    ops_vs.iter().find_map(|x| match x {
                        AdmotionOption::Title(title) => Some(*title),
                        _ => None,
                    })
                })
                .flatten()
                .to_owned(),
            show_icon: ops
                .as_ref()
                .map(|ops_vs| {
                    ops_vs.iter().find_map(|x| match x {
                        AdmotionOption::ShowIcon(v) => Some(*v),
                        _ => None,
                    })
                })
                .flatten()
                .unwrap_or(true),
            content,
        })
}

fn build_atlassian_markup_parser<'a>() -> impl Parser<'a, &'a str, Vec<MarkUpNode<'a>>> {
    let heading = build_atlassian_markup_heading_parser();
    heading.repeated().collect()
}

pub fn transpile_atlassian_markup_to_markdown(atlassian_markup: &str) -> String {
    let atlassian_markup_ast = build_atlassian_markup_parser()
        .parse(atlassian_markup)
        .unwrap();
    atlassian_markup_ast.iter().fold(
        String::with_capacity(atlassian_markup.len() * 2),
        |acc, node| acc + node.to_markdown_string().as_str(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::Parser;
    use yare::parameterized;

    #[test]
    fn parse_heading_h1() {
        let parser = build_atlassian_markup_heading_parser();
        let parsed = parser.parse("h1. Some heading\n").unwrap();
        assert!(matches!(parsed, MarkUpNode::Heading1(_)));
    }

    #[test]
    fn parse_heading_h2() {
        let parser = build_atlassian_markup_heading_parser();
        let parsed = parser.parse("h2. Some heading\n").unwrap();
        assert!(matches!(parsed, MarkUpNode::Heading2(_)));
    }

    #[test]
    fn parse_heading_h3() {
        let parser = build_atlassian_markup_heading_parser();
        let parsed = parser.parse("h3. Some heading\n").unwrap();
        assert!(matches!(parsed, MarkUpNode::Heading3(_)));
    }
    #[test]
    fn parse_heading_h4() {
        let parser = build_atlassian_markup_heading_parser();
        let parsed = parser.parse("h4. Some heading\n").unwrap();
        assert!(matches!(parsed, MarkUpNode::Heading4(_)));
    }
    #[test]
    fn parse_heading_h5() {
        let parser = build_atlassian_markup_heading_parser();
        let parsed = parser.parse("h5. Some heading\n").unwrap();
        assert!(matches!(parsed, MarkUpNode::Heading5(_)));
    }

    #[test]
    fn parse_heading_h6() {
        let parser = build_atlassian_markup_heading_parser();
        let parsed = parser.parse("h6. Some heading\n").unwrap();
        assert!(matches!(parsed, MarkUpNode::Heading6(_)));
    }

    #[parameterized(
            h1 = {"h1. Some heading\n", "# Some heading\n"},
            h2 = {"h2. Some heading\n", "## Some heading\n"},
            h3 = {"h3. Some heading\n", "### Some heading\n"},
            h4 = {"h4. Some heading\n", "#### Some heading\n"},
            h5 = {"h5. Some heading\n", "##### Some heading\n"},
            h6 = {"h6. Some heading\n", "###### Some heading\n"},
        )]
    fn translates_headings(am_heading_line: &str, md_heading_line: &str) {
        let parser = transpile_atlassian_markup_to_markdown(am_heading_line);
        assert_eq!(parser, md_heading_line);
    }

    #[test]
    fn parse_codeblock_all_params() {
        let markup = "{code:title=This is my title|linenumbers=true|language=python|firstline=0001|collapse=true}
    This is my code
    {code}
    ";
        let parsed = build_code_block_parser().parse(markup).unwrap();
        assert_eq!(
            parsed,
            MarkUpNode::CodeBlock {
                language: Some("python"),
                content: "This is my code\n",
            }
        );
    }

    #[test]
    fn parse_codeblock_with_title_language() {
        let markup = "{code:title=This|language=python}
    This is my code
    {code}
    ";
        let parsed = build_code_block_parser().parse(markup).unwrap();
        assert_eq!(
            parsed,
            MarkUpNode::CodeBlock {
                language: Some("python"),
                content: "This is my code\n",
            }
        );
    }

    #[test]
    fn parse_codeblock_with_language() {
        let markup = "{code:language=python}
    This is my code
    {code}
    ";
        let parsed = build_code_block_parser().parse(markup).unwrap();
        assert_eq!(
            parsed,
            MarkUpNode::CodeBlock {
                language: Some("python"),
                content: "This is my code\n",
            }
        );
    }

    #[test]
    fn parse_codeblock_no_params() {
        let markup = "{code}
    This is my code
    {code}
    ";
        let parsed = build_code_block_parser().parse(markup).unwrap();
        assert_eq!(
            parsed,
            MarkUpNode::CodeBlock {
                language: None,
                content: "This is my code\n",
            }
        );
    }

    #[parameterized(
            info_no_opts = {
                "{info}\nSome content\n{info}",
                AdmotionKind::Info,
                MarkUpNode::Admotion{kind: AdmotionKind::Info, title: None, show_icon: true, content: "Some content\n"}
            },
            info_only_title = {
                "{info:title=My title}\nSome content\n{info}",
                AdmotionKind::Info,
                MarkUpNode::Admotion{kind: AdmotionKind::Info, title: Some("My title"), show_icon: true, content: "Some content\n"}
            },
            info_only_icon = {
            "{info:show_icon=false}\nSome content\n{info}",
            AdmotionKind::Info,
            MarkUpNode::Admotion{kind: AdmotionKind::Info, title: None, show_icon: false, content: "Some content\n"}
        },
        info_all_opts = {
            "{info:title=My title|show_icon=false}\nSome content\n{info}",
            AdmotionKind::Info,
            MarkUpNode::Admotion{kind: AdmotionKind::Info, title: Some("My title"), show_icon: false, content: "Some content\n"}
        },
        info_all_opts_reversed = {
            "{info:show_icon=false|title=My title}\nSome content\n{info}",
            AdmotionKind::Info,
            MarkUpNode::Admotion{kind: AdmotionKind::Info, title: Some("My title"), show_icon: false, content: "Some content\n"}
        },

        warning_no_opts = {
            "{warning}\nSome content\n{warning}",
            AdmotionKind::Warning,
            MarkUpNode::Admotion{kind: AdmotionKind::Warning, title: None, show_icon: true, content: "Some content\n"}
        },
        warning_only_title = {
            "{warning:title=My title}\nSome content\n{warning}",
            AdmotionKind::Warning,
            MarkUpNode::Admotion{kind: AdmotionKind::Warning, title: Some("My title"), show_icon: true, content: "Some content\n"}
        },
        warning_only_icon = {
            "{warning:show_icon=false}\nSome content\n{warning}",
            AdmotionKind::Warning,
            MarkUpNode::Admotion{kind: AdmotionKind::Warning, title: None, show_icon: false, content: "Some content\n"}
        },
        warning_all_opts = {
            "{warning:title=My title|show_icon=false}\nSome content\n{warning}",
            AdmotionKind::Warning,
            MarkUpNode::Admotion{kind: AdmotionKind::Warning, title: Some("My title"), show_icon: false, content: "Some content\n"}
        },
        warning_all_opts_reversed = {
            "{warning:show_icon=false|title=My title}\nSome content\n{warning}",
            AdmotionKind::Warning,
            MarkUpNode::Admotion{kind: AdmotionKind::Warning, title: Some("My title"), show_icon: false, content: "Some content\n"}
        },

        tip_no_opts = {
            "{tip}\nSome content\n{tip}",
            AdmotionKind::Tip,
            MarkUpNode::Admotion{kind: AdmotionKind::Tip, title: None, show_icon: true, content: "Some content\n"}
        },
        tip_only_title = {
            "{tip:title=My title}\nSome content\n{tip}",
            AdmotionKind::Tip,
            MarkUpNode::Admotion{kind: AdmotionKind::Tip, title: Some("My title"), show_icon: true, content: "Some content\n"}
        },
        tip_only_icon = {
            "{tip:show_icon=false}\nSome content\n{tip}",
            AdmotionKind::Tip,
            MarkUpNode::Admotion{kind: AdmotionKind::Tip, title: None, show_icon: false, content: "Some content\n"}
        },
        tip_all_opts = {
            "{tip:title=My title|show_icon=false}\nSome content\n{tip}",
            AdmotionKind::Tip,
            MarkUpNode::Admotion{kind: AdmotionKind::Tip, title: Some("My title"), show_icon: false, content: "Some content\n"}
        },
        tip_all_opts_reversed = {
            "{tip:show_icon=false|title=My title}\nSome content\n{tip}",
            AdmotionKind::Tip,
            MarkUpNode::Admotion{kind: AdmotionKind::Tip, title: Some("My title"), show_icon: false, content: "Some content\n"}
        },

        note_no_opts = {
            "{note}\nSome content\n{note}",
            AdmotionKind::Note,
            MarkUpNode::Admotion{kind: AdmotionKind::Note, title: None, show_icon: true, content: "Some content\n"}
        },
        note_only_title = {
            "{note:title=My title}\nSome content\n{note}",
            AdmotionKind::Note,
            MarkUpNode::Admotion{kind: AdmotionKind::Note, title: Some("My title"), show_icon: true, content: "Some content\n"}
        },
        note_only_icon = {
            "{note:show_icon=false}\nSome content\n{note}",
            AdmotionKind::Note,
            MarkUpNode::Admotion{kind: AdmotionKind::Note, title: None, show_icon: false, content: "Some content\n"}
        },
        note_all_opts = {
            "{note:title=My title|show_icon=false}\nSome content\n{note}",
            AdmotionKind::Note,
            MarkUpNode::Admotion{kind: AdmotionKind::Note, title: Some("My title"), show_icon: false, content: "Some content\n"}
        },
        note_all_opts_reversed = {
            "{note:show_icon=false|title=My title}\nSome content\n{note}",
            AdmotionKind::Note,
            MarkUpNode::Admotion{kind: AdmotionKind::Note, title: Some("My title"), show_icon: false, content: "Some content\n"}
        },
    )]
    fn parse_info_macro(markup: &str, admotion_kind: AdmotionKind, target_node: MarkUpNode) {
        let parsed = build_admotion_parser(admotion_kind).parse(markup).unwrap();
        assert_eq!(parsed, target_node);
    }
}
