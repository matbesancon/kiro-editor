use std::iter;
use std::mem;

use crate::ansi_color::AnsiColor;
use crate::language::Language;
use crate::row::Row;

#[derive(Clone, Copy, PartialEq)]
pub enum Highlight {
    Normal,
    Number,
    String,
    Comment,
    Keyword,
    Type,
    Char,
    Statement,
    Match,
}

impl Highlight {
    pub fn color(self) -> AnsiColor {
        use AnsiColor::*;
        use Highlight::*;
        match self {
            Normal => Reset,
            Number => Red,
            String => Green,
            Comment => Gray,
            Keyword => Yellow,
            Type => Purple,
            Char => Green,
            Statement => Blue,
            Match => CyanUnderline,
        }
    }
}

struct SyntaxHighlight {
    lang: Language,
    string_quotes: &'static [char],
    number: bool,
    character: bool,
    line_comment: Option<&'static str>,
    block_comment: Option<(&'static str, &'static str)>,
    keywords: &'static [&'static str],
    control_statements: &'static [&'static str],
    builtin_types: &'static [&'static str],
}

const PLAIN_SYNTAX: SyntaxHighlight = SyntaxHighlight {
    lang: Language::Plain,
    number: false,
    string_quotes: &[],
    character: false,
    line_comment: None,
    block_comment: None,
    keywords: &[],
    control_statements: &[],
    builtin_types: &[],
};

const C_SYNTAX: SyntaxHighlight = SyntaxHighlight {
    lang: Language::C,
    number: true,
    string_quotes: &['"'],
    character: true,
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
    keywords: &[
        "auto", "const", "enum", "extern", "inline", "register", "restrict", "sizeof", "static",
        "struct", "typedef", "union", "volatile",
    ],
    control_statements: &[
        "break", "case", "continue", "default", "do", "else", "for", "goto", "if", "return",
        "switch", "while",
    ],
    builtin_types: &[
        "char", "double", "float", "int", "long", "short", "signed", "unsigned", "void",
    ],
};

const RUST_SYNTAX: SyntaxHighlight = SyntaxHighlight {
    lang: Language::Rust,
    number: true,
    string_quotes: &['"'],
    character: true,
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
    keywords: &[
        "as", "const", "crate", "dyn", "enum", "extern", "false", "fn", "impl", "let", "mod",
        "move", "mut", "pub", "ref", "Self", "self", "static", "struct", "super", "trait", "true",
        "type", "unsafe", "use", "where",
    ],
    control_statements: &[
        "break", "continue", "else", "for", "if", "in", "loop", "match", "return", "while",
    ],
    builtin_types: &[
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usuze",
        "f32", "f64", "bool", "char",
    ],
};

const JAVASCRIPT_SYNTAX: SyntaxHighlight = SyntaxHighlight {
    lang: Language::JavaScript,
    number: true,
    string_quotes: &['"', '\''],
    character: false,
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
    keywords: &[
        "class",
        "const",
        "debugger",
        "delete",
        "export",
        "extends",
        "function",
        "import",
        "in",
        "instanceof",
        "new",
        "super",
        "this",
        "typeof",
        "var",
        "void",
        "with",
        "yield",
    ],
    control_statements: &[
        "break", "case", "catch", "continue", "default", "do", "else", "finally", "for", "if",
        "return", "switch", "throw", "try", "while",
    ],
    builtin_types: &[
        "Object",
        "Function",
        "Boolean",
        "Symbol",
        "Error",
        "Number",
        "BigInt",
        "Math",
        "Date",
        "String",
        "RegExp",
        "Array",
        "Int8Array",
        "Int16Array",
        "Int32Array",
        "BigInt64Array",
        "Uint8Array",
        "Uint16Array",
        "Uint32Array",
        "BigUint64Array",
        "Float32Array",
        "Float64Array",
        "ArrayBuffer",
        "SharedArrayBuffer",
        "Atomics",
        "DataView",
        "JSON",
        "Promise",
        "Generator",
        "GeneratorFunction",
        "AsyncFunction",
        "Refrect",
        "Proxy",
        "Intl",
        "WebAssembly",
    ],
};

const GO_SYNTAX: SyntaxHighlight = SyntaxHighlight {
    lang: Language::Go,
    number: true,
    string_quotes: &['"'],
    character: true,
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
    keywords: &[
        "chan",
        "const",
        "defer",
        "func",
        "go",
        "import",
        "interface",
        "map",
        "package",
        "range",
        "struct",
        "type",
        "var",
    ],
    control_statements: &[
        "break",
        "case",
        "continue",
        "default",
        "else",
        "fallthrough",
        "for",
        "goto",
        "if",
        "return",
        "select",
        "switch",
    ],
    builtin_types: &[
        "bool",
        "byte",
        "complex128",
        "complex64",
        "error",
        "float32",
        "float64",
        "int",
        "int16",
        "int32",
        "int64",
        "int8",
        "rune",
        "string",
        "uint",
        "uint16",
        "uint32",
        "uint64",
        "uint8",
        "uintptr",
    ],
};

impl SyntaxHighlight {
    fn for_lang(lang: Language) -> &'static SyntaxHighlight {
        use Language::*;
        match lang {
            Plain => &PLAIN_SYNTAX,
            C => &C_SYNTAX,
            Rust => &RUST_SYNTAX,
            JavaScript => &JAVASCRIPT_SYNTAX,
            Go => &GO_SYNTAX,
        }
    }
}

pub struct Highlighting {
    pub needs_update: bool,
    pub lines: Vec<Vec<Highlight>>, // One item per one character
    previous_bottom_of_screen: usize,
    matched: Option<(usize, usize, Vec<Highlight>)>, // (x, y, saved)
    syntax: &'static SyntaxHighlight,
}

impl Default for Highlighting {
    fn default() -> Self {
        Highlighting {
            needs_update: false,
            lines: vec![],
            previous_bottom_of_screen: 0,
            matched: None,
            syntax: &PLAIN_SYNTAX,
        }
    }
}

impl Highlighting {
    pub fn new<'a, R: Iterator<Item = &'a Row>>(lang: Language, iter: R) -> Highlighting {
        Highlighting {
            needs_update: true,
            lines: iter
                .map(|r| {
                    iter::repeat(Highlight::Normal)
                        .take(r.render.len())
                        .collect()
                })
                .collect(),
            previous_bottom_of_screen: 0,
            matched: None,
            syntax: SyntaxHighlight::for_lang(lang),
        }
    }

    pub fn lang_changed(&mut self, new_lang: Language) {
        if self.syntax.lang == new_lang {
            return;
        }
        self.syntax = SyntaxHighlight::for_lang(new_lang);
        self.needs_update = true;
    }

    pub fn update(&mut self, rows: &[Row], bottom_of_screen: usize) {
        if !self.needs_update && bottom_of_screen <= self.previous_bottom_of_screen {
            return;
        }

        self.lines.resize_with(rows.len(), Default::default);

        fn is_sep(c: char) -> bool {
            c.is_ascii_whitespace() || (c.is_ascii_punctuation() && c != '_') || c == '\0'
        }

        fn starts_with_word(input: &str, word: &str) -> bool {
            if !input.starts_with(word) {
                return false;
            }

            let word_len = word.len();
            if input.len() == word_len {
                return true;
            }

            if let Some(c) = input.chars().nth(word_len) {
                is_sep(c)
            } else {
                false
            }
        }

        let mut prev_quote = None;
        let mut in_block_comment = false;
        for (y, ref row) in rows.iter().enumerate().take(bottom_of_screen) {
            self.lines[y].resize(row.render.len(), Highlight::Normal);

            let mut prev_hl = Highlight::Normal;
            let mut prev_char = '\0';
            let mut iter = row.render.char_indices().enumerate();

            while let Some((x, (idx, c))) = iter.next() {
                let mut hl = Highlight::Normal;

                if self.lines[y][x] == Highlight::Match {
                    hl = Highlight::Match;
                }

                if let Some((comment_start, comment_end)) = self.syntax.block_comment {
                    if hl == Highlight::Normal && prev_quote.is_none() {
                        let comment_delim = if in_block_comment
                            && row.render[idx..].starts_with(comment_end)
                        {
                            in_block_comment = false;
                            Some(comment_end)
                        } else if !in_block_comment && row.render[idx..].starts_with(comment_start)
                        {
                            in_block_comment = true;
                            Some(comment_start)
                        } else {
                            None
                        };

                        // Eat delimiter of block comment at once
                        if let Some(comment_delim) = comment_delim {
                            // Consume whole '/*' here. Otherwise such as '/*/' is wrongly accepted
                            let len = comment_delim.len();
                            self.lines[y]
                                .splice(x..x + len, iter::repeat(Highlight::Comment).take(len));

                            prev_hl = Highlight::Comment;
                            prev_char = comment_delim.chars().last().unwrap();
                            iter.nth(len - 2);
                            continue;
                        }

                        if in_block_comment {
                            hl = Highlight::Comment;
                        }
                    }
                }

                if let Some(comment_leader) = self.syntax.line_comment {
                    if prev_quote.is_none() && row.render[idx..].starts_with(comment_leader) {
                        let len = self.lines[y].len();
                        self.lines[y].splice(x.., iter::repeat(Highlight::Comment).take(len - x));
                        break;
                    }
                }

                if hl == Highlight::Normal && self.syntax.character {
                    let mut i = row.render[idx..].chars();
                    let len = match (i.next(), i.next(), i.next(), i.next()) {
                        (Some('\''), Some('\\'), _, Some('\'')) => Some(4),
                        (Some('\''), _, Some('\''), _) => Some(3),
                        _ => None,
                    };

                    if let Some(len) = len {
                        self.lines[y].splice(x..x + len, iter::repeat(Highlight::Char).take(len));
                        prev_hl = Highlight::Char;
                        prev_char = '\'';
                        iter.nth(len - 2);
                        continue;
                    }
                }

                if hl == Highlight::Normal && !self.syntax.string_quotes.is_empty() {
                    if let Some(q) = prev_quote {
                        // In string literal. XXX: "\\" is not highlighted correctly
                        if prev_char != '\\' && q == c {
                            prev_quote = None;
                        }
                        hl = Highlight::String;
                    } else if self.syntax.string_quotes.contains(&c) {
                        prev_quote = Some(c);
                        hl = Highlight::String;
                    }
                }

                let is_bound = is_sep(prev_char) ^ is_sep(c);

                // Highlight identifiers
                if hl == Highlight::Normal && is_bound {
                    let line = &row.render[idx..];
                    if let Some((keyword, highlight)) = self
                        .syntax
                        .keywords
                        .iter()
                        .zip(iter::repeat(Highlight::Keyword))
                        .chain(
                            self.syntax
                                .control_statements
                                .iter()
                                .zip(iter::repeat(Highlight::Statement)),
                        )
                        .chain(
                            self.syntax
                                .builtin_types
                                .iter()
                                .zip(iter::repeat(Highlight::Type)),
                        )
                        .find(|(k, _)| starts_with_word(line, k))
                    {
                        let len = keyword.len();
                        self.lines[y].splice(x..x + len, iter::repeat(highlight).take(len));

                        prev_hl = highlight;
                        prev_char = line.chars().nth(len - 1).unwrap();
                        // Consume keyword from input. `- 2` because first character was already
                        // consumed by the while statement
                        iter.nth(len - 2);

                        continue;
                    }
                }

                if hl == Highlight::Normal
                    && self.syntax.number
                    && (c.is_ascii_digit() && (prev_hl == Highlight::Number || is_bound)
                        || c == '.' && prev_hl == Highlight::Number)
                {
                    hl = Highlight::Number;
                }

                self.lines[y][x] = hl;
                prev_hl = hl;
                prev_char = c;
            }
        }

        self.needs_update = false;
        self.previous_bottom_of_screen = bottom_of_screen;
    }

    pub fn set_match(&mut self, y: usize, start: usize, end: usize) {
        if start >= end {
            return;
        }
        self.clear_previous_match();
        self.matched = Some((start, y, self.lines[y][start..end].to_vec()));
        self.lines[y].splice(start..end, iter::repeat(Highlight::Match).take(end - start));
    }

    pub fn clear_previous_match(&mut self) -> Option<usize> {
        if let Some((x, y, saved)) = mem::replace(&mut self.matched, None) {
            // Restore previously replaced highlights
            self.lines[y].splice(x..(x + saved.len()), saved.into_iter());
            Some(y)
        } else {
            None
        }
    }
}
