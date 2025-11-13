use std::path::Path;

pub struct FileType {
    name: String,
    hl_opts: HighlightingOptions,
}

#[derive(Default)]

pub struct HighlightingOptions {
    numbers: bool,

    strings: bool,

    characters: bool,

    comments: bool,

    multiline_comments: bool,

    primary_keywords: Vec<String>,

    secondary_keywords: Vec<String>,
}

impl Default for FileType {
    fn default() -> Self {
        Self {
            name: String::from("No filetype"),

            hl_opts: HighlightingOptions::default(),
        }
    }
}

impl FileType {
    #[must_use]

    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[must_use]

    pub fn highlighting_options(&self) -> &HighlightingOptions {
        &self.hl_opts
    }

    #[must_use]

    pub fn from(file_name: &str) -> Self {
        // Lowercased helpers

        let file_path_str = file_name.to_string();

        let file_path_lower = file_path_str.to_ascii_lowercase();

        let basename_lower = Path::new(&file_path_str)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        let ext = Path::new(&file_path_str)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());

        // Simple wildcard matcher (* and ?)
        let matches_glob = |pattern: &str, text: &str| -> bool {
            fn helper(p: &[u8], t: &[u8]) -> bool {
                let (mut pi, mut ti) = (0usize, 0usize);

                let mut star: Option<usize> = None;

                let mut match_t: usize = 0;

                while ti < t.len() {
                    if pi < p.len() && (p[pi] == b'?' || p[pi] == t[ti]) {
                        pi += 1;

                        ti += 1;
                    } else if pi < p.len() && p[pi] == b'*' {
                        star = Some(pi);

                        match_t = ti;

                        pi += 1;
                    } else if let Some(si) = star {
                        pi = si + 1;

                        match_t += 1;

                        ti = match_t;
                    } else {
                        return false;
                    }
                }

                while pi < p.len() && p[pi] == b'*' {
                    pi += 1;
                }

                pi == p.len()
            }

            helper(pattern.as_bytes(), text.as_bytes())
        };

        // Candidate mapping files
        let candidates = [
            "wd40text/assets/filetypes.txt",
            "assets/filetypes.txt",
            "filetypes.txt",
        ];

        let mut contents_opt = None;

        for p in &candidates {
            if let Ok(c) = std::fs::read_to_string(p) {
                contents_opt = Some(c);

                break;
            }
        }

        if let Some(contents) = contents_opt {
            for raw_line in contents.lines() {
                let mut line = raw_line.trim();

                if line.is_empty() {
                    continue;
                }

                if let Some((pre, _)) = line.split_once('#') {
                    line = pre.trim();

                    if line.is_empty() {
                        continue;
                    }
                }

                // Delimiters: =>, ->, :, =
                let (lhs, rhs) = if let Some((l, r)) = line.split_once("=>") {
                    (l.trim(), r.trim())
                } else if let Some((l, r)) = line.split_once("->") {
                    (l.trim(), r.trim())
                } else if let Some((l, r)) = line.split_once(':') {
                    (l.trim(), r.trim())
                } else if let Some((l, r)) = line.split_once('=') {
                    (l.trim(), r.trim())
                } else {
                    continue;
                };

                if lhs.is_empty() || rhs.is_empty() {
                    continue;
                }

                let display_name = rhs.trim().trim_matches(|c| c == '"' || c == '\'');

                let mut matched = false;

                for token in lhs.split([',', ';']) {
                    let token = token.trim();

                    if token.is_empty() {
                        continue;
                    }

                    for part in token.split_whitespace() {
                        let mut pat = part
                            .trim()
                            .trim_matches(|c| c == '"' || c == '\'')
                            .to_ascii_lowercase();

                        if pat.is_empty() {
                            continue;
                        }

                        if pat.starts_with('.') {
                            pat.remove(0);
                        }

                        if pat.contains('*')
                            || pat.contains('?')
                            || pat.contains('/')
                            || pat.contains('\\')
                        {
                            if matches_glob(&pat, &file_path_lower)
                                || matches_glob(&pat, &basename_lower)
                            {
                                matched = true;
                            }
                        } else if pat.contains('.') {
                            if basename_lower == pat {
                                matched = true;
                            }
                        } else if let Some(ref e) = ext {
                            if e == &pat {
                                matched = true;
                            }
                        }

                        if matched {
                            break;
                        }
                    }

                    if matched {
                        let name = display_name.to_string();

                        if ext.as_deref() == Some("rs") {
                            return Self {
                                name,

                                hl_opts: HighlightingOptions {
                                    numbers: true,

                                    strings: true,

                                    characters: true,

                                    comments: true,

                                    multiline_comments: true,

                                    primary_keywords: vec![
                                        "as".into(),
                                        "break".into(),
                                        "const".into(),
                                        "continue".into(),
                                        "crate".into(),
                                        "else".into(),
                                        "enum".into(),
                                        "extern".into(),
                                        "false".into(),
                                        "fn".into(),
                                        "for".into(),
                                        "if".into(),
                                        "impl".into(),
                                        "in".into(),
                                        "let".into(),
                                        "loop".into(),
                                        "match".into(),
                                        "mod".into(),
                                        "move".into(),
                                        "mut".into(),
                                        "pub".into(),
                                        "ref".into(),
                                        "return".into(),
                                        "self".into(),
                                        "Self".into(),
                                        "static".into(),
                                        "struct".into(),
                                        "super".into(),
                                        "trait".into(),
                                        "true".into(),
                                        "type".into(),
                                        "unsafe".into(),
                                        "use".into(),
                                        "where".into(),
                                        "while".into(),
                                        "dyn".into(),
                                        "abstract".into(),
                                        "become".into(),
                                        "box".into(),
                                        "do".into(),
                                        "final".into(),
                                        "macro".into(),
                                        "override".into(),
                                        "priv".into(),
                                        "typeof".into(),
                                        "unsized".into(),
                                        "virtual".into(),
                                        "yield".into(),
                                        "async".into(),
                                        "await".into(),
                                        "try".into(),
                                    ],

                                    secondary_keywords: vec![
                                        "bool".into(),
                                        "char".into(),
                                        "i8".into(),
                                        "i16".into(),
                                        "i32".into(),
                                        "i64".into(),
                                        "isize".into(),
                                        "u8".into(),
                                        "u16".into(),
                                        "u32".into(),
                                        "u64".into(),
                                        "usize".into(),
                                        "f32".into(),
                                        "f64".into(),
                                    ],
                                },
                            };
                        } else {
                            return Self {
                                name,
                                hl_opts: HighlightingOptions::default(),
                            };
                        }
                    }
                }
            }
        }

        match ext.as_deref() {
            Some("rs") => Self {
                name: String::from("Rust"),

                hl_opts: HighlightingOptions {
                    numbers: true,

                    strings: true,

                    characters: true,

                    comments: true,

                    multiline_comments: true,

                    primary_keywords: vec![
                        "as".into(),
                        "break".into(),
                        "const".into(),
                        "continue".into(),
                        "crate".into(),
                        "else".into(),
                        "enum".into(),
                        "extern".into(),
                        "false".into(),
                        "fn".into(),
                        "for".into(),
                        "if".into(),
                        "impl".into(),
                        "in".into(),
                        "let".into(),
                        "loop".into(),
                        "match".into(),
                        "mod".into(),
                        "move".into(),
                        "mut".into(),
                        "pub".into(),
                        "ref".into(),
                        "return".into(),
                        "self".into(),
                        "Self".into(),
                        "static".into(),
                        "struct".into(),
                        "super".into(),
                        "trait".into(),
                        "true".into(),
                        "type".into(),
                        "unsafe".into(),
                        "use".into(),
                        "where".into(),
                        "while".into(),
                        "dyn".into(),
                        "abstract".into(),
                        "become".into(),
                        "box".into(),
                        "do".into(),
                        "final".into(),
                        "macro".into(),
                        "override".into(),
                        "priv".into(),
                        "typeof".into(),
                        "unsized".into(),
                        "virtual".into(),
                        "yield".into(),
                        "async".into(),
                        "await".into(),
                        "try".into(),
                    ],
                    secondary_keywords: vec![
                        "bool".into(),
                        "char".into(),
                        "i8".into(),
                        "i16".into(),
                        "i32".into(),
                        "i64".into(),
                        "isize".into(),
                        "u8".into(),
                        "u16".into(),
                        "u32".into(),
                        "u64".into(),
                        "usize".into(),
                        "f32".into(),
                        "f64".into(),
                    ],
                },
            },
            Some("doc") => Self {
                name: "MS Word 95-97".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("docx") => Self {
                name: "MS Word".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("txt") => Self {
                name: "Plain Text".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("odt") => Self {
                name: "OpenDocument Text".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("gd") => Self {
                name: "GDScript".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("tscn") => Self {
                name: "Godot Scene".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("scn") => Self {
                name: "Godot Scene (binary)".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("tres") => Self {
                name: "Godot Resource".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("res") => Self {
                name: "Godot Resource (binary)".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("gdshader") => Self {
                name: "Godot Shader".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("shader") => Self {
                name: "Shader".into(),
                hl_opts: HighlightingOptions::default(),
            },
            Some("godot") => Self {
                name: "Godot Project".into(),
                hl_opts: HighlightingOptions::default(),
            },
            _ => Self::default(),
        }
    }
}

impl HighlightingOptions {
    #[must_use]

    pub fn numbers(&self) -> bool {
        self.numbers
    }

    #[must_use]

    pub fn strings(&self) -> bool {
        self.strings
    }

    #[must_use]

    pub fn characters(&self) -> bool {
        self.characters
    }

    #[must_use]

    pub fn comments(&self) -> bool {
        self.comments
    }

    #[must_use]

    pub fn primary_keywords(&self) -> &Vec<String> {
        &self.primary_keywords
    }

    #[must_use]

    pub fn secondary_keywords(&self) -> &Vec<String> {
        &self.secondary_keywords
    }

    #[must_use]

    pub fn multiline_comments(&self) -> bool {
        self.multiline_comments
    }
}
