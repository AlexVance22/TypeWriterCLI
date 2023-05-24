use std::{
    fs,
    fmt::Write,
};
use thiserror::Error;
use crate::CmdInfo;


#[derive(Error, Debug)]
pub enum HtmlError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    FormatError(#[from] std::fmt::Error),
    #[error("line {linenum} - invalid syntax (expected {expected} after {after})")]
    SyntaxError {
        linenum: usize,
        expected: String,
        after: String,
    },
    #[error("unknown html conversion error")]
    Unknown,
}


pub fn strip_empty<'a>((_, line): &'a (usize, &str)) -> bool {
    !(line.is_empty() || line.starts_with("* "))
}

pub fn trim_ignored<'a>((num, line): (usize, &str)) -> (usize, &str) {
    if line.contains("* ") {
        (num, line.split("* ").next().unwrap())
    } else {
        (num, line)
    }
}


pub struct Segments<'a> {
    lines: std::vec::IntoIter<(usize, &'a str)>,
    term: bool
}

impl<'a> Segments<'a> {
    pub fn new(src: &'a str) -> Self {
        Self{ lines: src.lines()
                        .enumerate()
                        .filter(strip_empty)
                        .map(trim_ignored)
                        .collect::<Vec<(usize, &'a str)>>()
                        .into_iter(),
            term: false
        }
        
    }

    fn next_as_line(&mut self) -> Option<(usize, Vec<&'a str>)> {
        if self.term == true { return None }

        let mut result: Vec<&str> = Vec::new();
        let (mut linenum, mut line) = self.lines.next()?;
        
        if line.trim() == "***" {
            return None
        }

        while let Some(trimmed) = line.strip_suffix("\\") {
            result.push(trimmed);
            (linenum, line) = self.lines.next()?;
            if line.trim() == "***" {
                self.term = true;
                return Some((linenum, result))
            }
        }

        result.push(line);

        Some((linenum, result))
    }
}

impl<'a> Iterator for Segments<'a> {
    type Item = (usize, Vec<&'a str>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.term == true { return None }

        let mut result: Vec<&str> = Vec::new();
        let (mut linenum, mut line) = self.lines.next()?;
        
        if line.trim() == "***" {
            return None
        }

        while let Some(trimmed) = line.strip_suffix("\\") {
            result.extend(trimmed.split_ascii_whitespace());
            (linenum, line) = self.lines.next()?;
            if line.trim() == "***" {
                self.term = true;
                return Some((linenum, result))
            }
        }

        result.extend(line.split_ascii_whitespace());

        Some((linenum, result))
    }
}


fn get_line(toks: &[&str], linenum: usize, scene: &mut u32, title: &str) -> Result<String, HtmlError> {
    let (kind, iter) = toks.split_first().ok_or(HtmlError::SyntaxError { linenum, expected: "mode declaration".to_string(), after: "new line".to_string() })?;
    let text = iter.join(" ").replace("$title", title);

    match *kind {
        "montage" if  text.is_empty() => Ok("<div class=\"header\">BEGIN MONTAGE:</div>\n".to_string()),
        "mon-end" if  text.is_empty() => Ok("<div class=\"header\">END MONTAGE.</div>\n".to_string()),
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
            let whole = toks.join(" ");

            if whole.contains(": (") {
                if let Some(i) = whole.chars().position(|c| c == '(') {
                    if let Some(j) = whole.chars().position(|c| c == ')') {
                        if i < j {
                            let name = whole[0..(i-2)].to_uppercase();
                            let paren = &whole[(i+1)..j];
                            let text = &whole[(j+2)..whole.len()];
                            return Ok(format!("<div class=\"name\">{name}</div>\n<div class=\"parens\">({paren})</div>\n<div class=\"speech\">{text}</div>\n"))
                        }
                    }
                }
                Err(HtmlError::SyntaxError { linenum, expected: "parenthetical".to_string(), after: "".to_string() })
            } else if whole.contains(": ") {
                let i = whole.chars().position(|c| c == ':').unwrap();
                let name = whole[0..i].to_uppercase();
                let text = &whole[(i+1)..whole.len()];
                Ok(format!("<div class=\"name\">{name}</div>\n<div class=\"speech\">{text}</div>\n"))
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
        match segment.len() {
            0 => unreachable!(),
            _ => {
                let line = get_line(&segment, linenum, &mut scene, &title)?;
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
        }
    }

    let html = format!("{start}{content}{end}");

    if cmd.temp {
        fs::write(format!("{}.html", cmd.file_root), &html)?;
    }

    Ok(fs::write(&cmd.html, html)?)
}
