use chumsky::{prelude::*, text::digits};
use regex::RegexBuilder;

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
}
