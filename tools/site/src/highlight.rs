use rune::highlight::{highlight, Class};
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub struct Highlighter {
    syntaxes: SyntaxSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
        }
    }

    /// A code block. Rune is classified with the same classes the editor uses,
    /// everything else by syntect with its scopes prefixed by `hl-`.
    pub fn block(&self, lang: &str, text: &str) -> String {
        let text = text.strip_suffix('\n').unwrap_or(text);

        let body = match lang {
            "rune" => rune(text),
            "" => escape(text),
            lang => self.syntect(lang, text),
        };

        let lang = match lang {
            "" => String::new(),
            lang => format!(" data-lang=\"{}\"", escape(lang)),
        };

        format!("<pre class=\"code-block\"{lang}><code>{body}</code></pre>")
    }

    fn syntect(&self, lang: &str, text: &str) -> String {
        let Some(syntax) = self.syntaxes.find_syntax_by_token(lang) else {
            return escape(text);
        };

        let style = ClassStyle::SpacedPrefixed { prefix: "hl-" };
        let mut generator =
            ClassedHTMLGenerator::new_with_class_style(syntax, &self.syntaxes, style);

        for line in LinesWithEndings::from(text) {
            if generator
                .parse_html_for_line_which_includes_newline(line)
                .is_err()
            {
                return escape(text);
            }
        }

        generator.finalize()
    }
}

fn rune(text: &str) -> String {
    let Ok(tokens) = highlight(text) else {
        return escape(text);
    };

    let mut out = String::new();

    for token in tokens.iter() {
        let Some(segment) = text.get(token.span.start.into_usize()..token.span.end.into_usize())
        else {
            continue;
        };

        if token.class == Class::Whitespace {
            out.push_str(&escape(segment));
        } else {
            out.push_str(&format!(
                "<span class=\"rune-token {}\">{}</span>",
                token.class.as_str(),
                escape(segment)
            ));
        }
    }

    out
}

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }

    out
}
