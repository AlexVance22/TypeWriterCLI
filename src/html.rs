use std::{
    fs,
    fmt::Write,
};
use lazy_static::lazy_static;
use regex::Regex;
use thiserror::Error;
use crate::CmdInfo;


#[derive(Error, Debug)]
pub enum HtmlError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    FormatError(#[from] std::fmt::Error),
    #[error("line {linenum} - invalid syntax (expected {expected} after {after})")]
    SyntaxError{
        linenum: usize,
        expected: String,
        after: String,
    },
    #[error("unknown html conversion error")]
    Unknown,
}


pub fn trim_ignored((num, line): (usize, &str)) -> (usize, &str) {
    if line.contains("* ") {
        (num, line.split("* ").next().unwrap())
    } else {
        (num, line)
    }
}

pub fn strip_empty((_, line): &(usize, &str)) -> bool {
    !line.trim().is_empty()
}


pub struct Segments<'a> {
    lines: std::vec::IntoIter<(usize, &'a str)>,
    term: bool
}

impl<'a> Segments<'a> {
    pub fn new(src: &'a str) -> Self {
        Self{ lines: src.lines()
                        .enumerate()
                        .map(trim_ignored)
                        .filter(strip_empty)
                        .collect::<Vec<(usize, &'a str)>>()
                        .into_iter(),
            term: false
        }
    }

    fn next_as_line(&mut self) -> Option<(usize, Vec<&'a str>)> {
        if self.term { return None }

        let (linenum, mut line) = self.lines.next()?;
        
        if line.trim() == "***" {
            return None
        }

        let mut result = Vec::new();

        while let Some(trimmed) = line.strip_suffix('\\') {
            result.push(trimmed.trim());
            (_, line) = self.lines.next()?;
            if line.trim() == "***" {
                self.term = true;
                return Some((linenum, result))
            }
        }

        result.push(line.trim());

        Some((linenum, result))
    }
}

impl<'a> Iterator for Segments<'a> {
    type Item = (usize, (&'a str, Vec<&'a str>));

    fn next(&mut self) -> Option<Self::Item> {
        if self.term { return None }

        let (linenum, line) = self.lines.next()?;
        
        if line.trim() == "***" {
            return None
        }

        let mut result = Vec::new();
        let (kind, mut line) = match line.split_once(char::is_whitespace) {
            Some(vals) => vals,
            None => return Some((linenum + 1, (line.trim(), result))),
        };

        while let Some(trimmed) = line.strip_suffix('\\') {
            result.push(trimmed.trim());
            (_, line) = self.lines.next()?;
            if line.trim() == "***" {
                self.term = true;
                return Some((linenum + 1, (kind.trim(), result)))
            }
        }

        result.push(line.trim());

        Some((linenum + 1, (kind.trim(), result)))
    }
}


fn get_line(toks: (&str, Vec<&str>), linenum: usize, scene: &mut u32, title: &str) -> Result<String, HtmlError> {
    lazy_static! {
        static ref PAT_SCENE: Regex = Regex::new(r"(INT\.|EXT\.) [^a-z]+ - [^a-z]+").unwrap();
        static ref PAT_SPEECH: Regex = Regex::new(r"([a-z]+(?: \((?:O\.S\.|V\.O\.)\))?):\s+(?:(\([A-Z][^\)]*\) )?([^\(]+))+").unwrap();
        static ref PAT_EXTRACT: Regex = Regex::new(r"\s*(\([^\)]+\))?((?:\s+[^\(]+)+)").unwrap();
    }

    let (kind, text) = toks;
    let text = text.join(" ").replace("$title", title);

    match kind {
        "montage" if  text.is_empty() => Ok("<div class=\"header\">BEGIN MONTAGE:</div>\n".to_string()),
        "mon-end" if  text.is_empty() => Ok("<div class=\"header\">END MONTAGE.</div>\n".to_string()),
        "TODO"    if  text.is_empty() => Ok("<div class=\"header\">TODO =========================</div>\n".to_string()),
        "direct"  if !text.is_empty() => Ok(format!("<div class=\"direct\">{text}</div>\n")),
        "parens"  if !text.is_empty() => Ok(format!("<div class=\"parens\">({text})</div>\n")),
        "speech"  if !text.is_empty() => Ok(format!("<div class=\"speech\">{text}</div>\n")),
        "subhead" if !text.is_empty() => Ok(format!("<div class=\"header\"><h2>{}</h2></div>\n", text.to_uppercase())),
        "trans"   if !text.is_empty() => Ok(format!("<div class=\"trans\">{}</div>\n", text.to_uppercase())),
        "chyron"  if !text.is_empty() => Ok(format!("<div class=\"direct\">CHYRON: {text}</div>\n")),
        "scene"   if !text.is_empty() => {
            *scene += 1;
            let count = 4 - scene.to_string().len();
            let mut pad = String::new();
            for _ in 0..count {
                write!(pad, "&nbsp;")?;
            }
            Ok(format!("<div class=\"scene\"><h1>{pad}{scene} {}</h1></div>\n", text.to_uppercase()))
        }

        "montage"|"mon-end" => {
            Err(HtmlError::SyntaxError{ linenum, expected: "newline".to_string(), after: format!("mode declaration '{kind}'") })
        }
        "direct"|"parens"|"speech"|"subhead"|"trans"|"chyron"|"scene" => {
            Err(HtmlError::SyntaxError{ linenum, expected: "content".to_string(), after: format!("mode declaration '{kind}'") })
        }

        _ => {
            let whole = format!("{} {}", kind, text).trim().to_string();

            if PAT_SCENE.is_match(&whole) {
                let whole = whole.strip_prefix("scene").unwrap_or(&whole).trim().to_string();
                *scene += 1;
                let count = 4 - scene.to_string().len();
                let mut pad = String::new();
                for _ in 0..count {
                    write!(pad, "&nbsp;")?;
                }
                Ok(format!("<div class=\"scene\"><h1>{pad}{scene} {}</h1></div>\n", whole))
            } else if PAT_SPEECH.is_match(&whole) {
                let (name, content) = whole.split_once(':').unwrap();
                let mut result = String::new();
                writeln!(result, "<div class=\"name\">{}</div>", name.to_ascii_uppercase())?;
                for pair in PAT_EXTRACT.captures_iter(content) {
                    for cap in pair.iter().skip(1).flatten() {
                        if cap.as_str().starts_with('(') {
                            writeln!(result, "<div class=\"parens\">{}</div>", cap.as_str())?;
                        } else {
                            writeln!(result, "<div class=\"speech\">{}</div>", cap.as_str())?;
                        }
                    }
                }
                Ok(result)
            } else {
                Err(HtmlError::SyntaxError { linenum, expected: "mode declaration".to_string(), after: "new line".to_string() })
            }
        }
    }
}


pub fn gen_html(cmd: &CmdInfo) -> Result<(), HtmlError> {
    let src = fs::read_to_string(&cmd.infile)?;
    let mut segments = Segments::new(&src);

    let title = segments.next_as_line().ok_or(HtmlError::SyntaxError { linenum: 1, expected: "title".to_string(), after: "beginning".to_string() })?.1.join(" ");
    let subtitle = segments.next_as_line().ok_or(HtmlError::SyntaxError { linenum: 1, expected: "subtitle".to_string(), after: "subtitle".to_string() })?.1.join(" ");
    let start = if cmd.range.is_some() {
        "<html><head><link rel=\"stylesheet\" href=\"../res/style.css\"/></head><body><div class=\"page\">\n".to_string()
    } else {
        format!("<html><head><link rel=\"stylesheet\" href=\"../res/style.css\"/></head><body><div class=\"page\">\n<div class=\"title\"><h1>{title}</h1></div>\n<div class=\"subtitle\"><h2>{subtitle}</h2></div>\n")
    };
    let end = "</div></body></html>";

    let mut content = String::new();
    let mut scene = 0u32;

    for (linenum, segment) in segments {
        let line = get_line(segment, linenum, &mut scene, &title)?;
        if let Some(range) = &cmd.range {
            if range.contains(&scene) {
                write!(content, "{}", line)?
            } else if scene > range.end {
                break
            }
        } else {
            write!(content, "{}", line)?
        }
    }

    let html = format!("{start}{content}{end}");

    if cmd.temp {
        fs::write(format!("{}.html", cmd.file_root), &html)?;
    }

    Ok(fs::write(&cmd.html, html)?)
}

