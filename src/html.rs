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
    #[error("line {line} - invalid syntax (expected {expected} after {after})")]
    SyntaxError{
        line: usize,
        expected: String,
        after: String,
    },
    #[error("unknown html conversion error")]
    Unknown,
}


pub fn trim_ignored((num, line): (usize, &str)) -> (usize, &str) {
    if line.contains("* ") {
        (num, line.split("* ").next().unwrap().trim())
    } else {
        (num, line.trim())
    }
}


struct Segment<'a> {
    line: usize,
    mode: &'a str,
    text: Vec<&'a str>,
}


struct Segments<'a> {
    lines: std::vec::IntoIter<(usize, &'a str)>,
    term: bool
}

impl<'a> Segments<'a> {
    fn new(src: &'a str) -> Self {
        Self{ lines: src.lines()
                        .enumerate()
                        .map(trim_ignored)
                        .filter(|(_, l)| !l.is_empty())
                        .collect::<Vec<(usize, &'a str)>>()
                        .into_iter(),
            term: false
        }
    }

    fn next_whole(&mut self) -> Option<(usize, Vec<&'a str>)> {
        if self.term { return None }

        let (line, mut val) = self.lines.next()?;
        
        if val == "***" { return None }

        let mut text = Vec::new();

        while let Some(strip) = val.strip_suffix('\\') {
            text.push(strip.trim());
            val = self.lines.next()?.1;
            if val == "***" {
                self.term = true;
                return Some((line + 1, text))
            }
        }

        text.push(val);

        Some((line + 1, text))
    }
}

impl<'a> Iterator for Segments<'a> {
    type Item = Segment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (line, mut text) = self.next_whole()?;
        let first = text.remove(0);

        if let Some((mode, rest)) = first.split_once(char::is_whitespace) {
            text.insert(0, rest.trim());
            Some(Segment{ line, mode, text })
        } else {
            Some(Segment{ line, mode: first, text: Vec::new() })
        }
    }
}


struct Context {
    scene: u32,
    title: String,
    subtitle: String,
}


fn get_line(segment: Segment, ctx: &mut Context) -> Result<String, HtmlError> {
    lazy_static! {
        static ref PAT_HEAD: Regex = Regex::new(r"^[^a-z]+$").unwrap();
        static ref PAT_SCENE: Regex = Regex::new(r"(INT\.|EXT\.) [^a-z]+ - [^a-z]+").unwrap();
        static ref PAT_SPEECH: Regex = Regex::new(r"(\w+(?: \((?:O\.S\.|V\.O\.)\))?):\s+(?:(\([A-Z][^\)]*\) )?([^\(]+))+").unwrap();
        static ref PAT_EXTRACT: Regex = Regex::new(r"\s*(\([^\)]+\))?((?:\s+[^\(]+)+)").unwrap();
    }

    let Segment{ line, mode, text } = segment;
    let text = text.join(" ")
                   .replace("$title", &ctx.title)
                   .replace("$subtitle", &ctx.subtitle);

    match mode {
        "montage" if  text.is_empty() => Ok("<div class=\"header\">BEGIN MONTAGE:</div>\n".to_string()),
        "mon-end" if  text.is_empty() => Ok("<div class=\"header\">END MONTAGE.</div>\n".to_string()),
        "TODO"    if  text.is_empty() => Ok("<div class=\"header\">TODO ==============================</div>\n".to_string()),
        "TODO"    if !text.is_empty() => Ok(format!("<div class=\"header\">TODO == {}</div>\n", text.to_uppercase())),
        "direct"  if !text.is_empty() => Ok(format!("<div class=\"direct\">{text}</div>\n")),
        "parens"  if !text.is_empty() => Ok(format!("<div class=\"parens\">({text})</div>\n")),
        "speech"  if !text.is_empty() => Ok(format!("<div class=\"speech\">{text}</div>\n")),
        "subhead" if !text.is_empty() => Ok(format!("<div class=\"header\"><h2>{}</h2></div>\n", text.to_uppercase())),
        "trans"   if !text.is_empty() => Ok(format!("<div class=\"trans\">{}</div>\n", text.to_uppercase())),
        "chyron"  if !text.is_empty() => Ok(format!("<div class=\"direct\">CHYRON: {text}</div>\n")),
        "scene"   if !text.is_empty() && PAT_SCENE.is_match(&text) => {
            ctx.scene += 1;
            let count = 4 - ctx.scene.to_string().len();
            let pad = vec!["&nbsp;"; count].join("");
            Ok(format!("<div class=\"scene\"><h1>{pad}{} {}</h1></div>\n", ctx.scene, text.to_uppercase()))
        }
        
        "montage"|"mon-end" => {
            Err(HtmlError::SyntaxError{ line, expected: "newline".to_string(), after: format!("mode declaration '{mode}'") })
        }
        "direct"|"parens"|"speech"|"subhead"|"trans"|"chyron" => {
            Err(HtmlError::SyntaxError{ line, expected: "content".to_string(), after: format!("mode declaration '{mode}'") })
        }
        "scene" => {
            Err(HtmlError::SyntaxError{ line, expected: "scene heading".to_string(), after: "scene declaration".to_string() })
        }

        _ => {
            let whole = format!("{} {}", mode, text).trim().to_string();

            if PAT_SCENE.is_match(&whole) {
                ctx.scene += 1;
                let count = 4 - ctx.scene.to_string().len();
                let pad = vec!["&nbsp;"; count].join("");
                Ok(format!("<div class=\"scene\"><h1>{pad}{} {}</h1></div>\n", ctx.scene, whole))
            } else if PAT_HEAD.is_match(&whole) {
                Ok(format!("<div class=\"header\">{}</div>", whole))
            } else if PAT_SPEECH.is_match(&whole) {
                let (name, content) = whole.split_once(':').unwrap();
                let mut result = String::new();
                writeln!(result, "<div class=\"name\">{}</div>", name.to_ascii_uppercase())?;
                for pair in PAT_EXTRACT.captures_iter(content) {
                    for cap in pair.iter().skip(1).flatten() {
                        if cap.as_str().starts_with('(') {
                            writeln!(result, "<div class=\"parens\">{}</div>", cap.as_str().trim())?;
                        } else {
                            writeln!(result, "<div class=\"speech\">{}</div>", cap.as_str().trim())?;
                        }
                    }
                }
                Ok(result)
            } else {
                Err(HtmlError::SyntaxError { line, expected: "mode declaration".to_string(), after: "new line".to_string() })
            }
        }
    }
}


pub fn gen_html(cmd: &CmdInfo) -> Result<(), HtmlError> {
    let src = fs::read_to_string(&cmd.infile)?;

    let mut segments = Segments::new(&src);
    let mut ctx = Context{
        scene: 0,
        title: segments.next_whole().ok_or(HtmlError::SyntaxError{ line: 1, expected: "title".to_string(), after: "beginning".to_string() })?.1.join(" "),
        subtitle: segments.next_whole().ok_or(HtmlError::SyntaxError{ line: 2, expected: "subtitle".to_string(), after: "title".to_string() })?.1.join(" "),
    };

    let mut result = if cmd.range.is_some() {
        "<html><head><link rel=\"stylesheet\" href=\"../res/style.css\"/></head><body><div class=\"page\">\n".to_string()
    } else {
        format!("<html><head><link rel=\"stylesheet\" href=\"../res/style.css\"/></head><body><div class=\"page\">\n\
                 <div class=\"title\"><h1>{}</h1></div>\n<div class=\"subtitle\"><h2>{}</h2></div>\n", ctx.title, ctx.subtitle)
    };

    for segment in segments {
        let line = get_line(segment, &mut ctx)?;
        if let Some(range) = &cmd.range {
            if range.contains(&ctx.scene) {
                result.push_str(&line);
            } else if ctx.scene > range.end {
                break
            }
        } else {
            result.push_str(&line);
        }
    }
    result.push_str("</div></body></html>");

    if cmd.temp {
        fs::write(format!("{}.html", cmd.file_root), &result)?;
    }

    Ok(fs::write(&cmd.html, result)?)
}



#[cfg(test)]
mod tests {
    use super::*;
    
    fn process(vals: &str) -> Vec<String> {
        let mut ctx = Context{ scene: 0, title: String::new(), subtitle: String::new() };

        Segments::new(vals)
            .map(|s| get_line(s, &mut ctx).expect("get line failed"))
            .collect()
    }

    #[test]
    fn comments() {
        let mut case = Segments::new(" \
            \n\
            line with content\n\
            line with comment * comment\n\
            \n\
            \n\
            * another comment");

        assert_eq!(case.next_whole(), Some((2, vec!["line with content"])));
        assert_eq!(case.next_whole(), Some((3, vec!["line with comment"])));
        assert_eq!(case.next_whole(), None);
    }

    #[test]
    fn line_reach() {
        let mut case = Segments::new(" \
            line with content \n\
            \n\
            line with some content \\ \n\
            and here some more content\n\
            \n\
            line with content \\ \n\
            more content \\ \n\
            last bit of content\n\
            \n\
            line with some content \\ * same as 2 but with comments \n\
            and here some more content \n\
            \n\
            line with some content * 2 lines because of comment \\ \n\
            and here some more content");

        assert_eq!(case.next_whole(), Some((1,  vec!["line with content"])));
        assert_eq!(case.next_whole(), Some((3,  vec!["line with some content", "and here some more content"])));
        assert_eq!(case.next_whole(), Some((6,  vec!["line with content", "more content", "last bit of content"])));
        assert_eq!(case.next_whole(), Some((10, vec!["line with some content", "and here some more content"])));
        assert_eq!(case.next_whole(), Some((13, vec!["line with some content"])));
        assert_eq!(case.next_whole(), Some((14, vec!["and here some more content"])));
        assert_eq!(case.next_whole(), None);
    }

    #[test]
    fn simple() {
        let cases = process(
            "direct this is directorial info\n\
             direct         how do i test this\n\
             trans  CUT TO\n\
             trans          CUT TO"
        );

        assert_eq!(cases[0], "<div class=\"direct\">this is directorial info</div>\n".to_string());
        assert_eq!(cases[1], "<div class=\"direct\">how do i test this</div>\n".to_string());
        assert_eq!(cases[2], "<div class=\"trans\">CUT TO</div>\n".to_string());
        assert_eq!(cases[3], "<div class=\"trans\">CUT TO</div>\n".to_string());
    }

    #[test]
    fn scenes() {
        let cases = process(
            "EXT. LOC - DAY\n\
                 EXT. LOC - DAY\n\
             INT. LOC WITH WORDS - TIME WITH WORDS\n\
             EXT. LOC WITH 123 - 12:25PM\n\
             scene EXT. LOC - DAY\n\
             scene    EXT. LOC - DAY\n\
             scene INT. LOC WITH WORDS - TIME WITH WORDS\n\
             scene EXT. LOC WITH 123 - 12:25PM"
        );

        assert_eq!(cases[0], "<div class=\"scene\"><h1>&nbsp;&nbsp;&nbsp;1 EXT. LOC - DAY</h1></div>\n".to_string());
        assert_eq!(cases[1], "<div class=\"scene\"><h1>&nbsp;&nbsp;&nbsp;2 EXT. LOC - DAY</h1></div>\n".to_string());
        assert_eq!(cases[2], "<div class=\"scene\"><h1>&nbsp;&nbsp;&nbsp;3 INT. LOC WITH WORDS - TIME WITH WORDS</h1></div>\n".to_string());
        assert_eq!(cases[3], "<div class=\"scene\"><h1>&nbsp;&nbsp;&nbsp;4 EXT. LOC WITH 123 - 12:25PM</h1></div>\n".to_string());
        assert_eq!(cases[4], "<div class=\"scene\"><h1>&nbsp;&nbsp;&nbsp;5 EXT. LOC - DAY</h1></div>\n".to_string());
        assert_eq!(cases[5], "<div class=\"scene\"><h1>&nbsp;&nbsp;&nbsp;6 EXT. LOC - DAY</h1></div>\n".to_string());
        assert_eq!(cases[6], "<div class=\"scene\"><h1>&nbsp;&nbsp;&nbsp;7 INT. LOC WITH WORDS - TIME WITH WORDS</h1></div>\n".to_string());
        assert_eq!(cases[7], "<div class=\"scene\"><h1>&nbsp;&nbsp;&nbsp;8 EXT. LOC WITH 123 - 12:25PM</h1></div>\n".to_string());
    }
    
    #[test]
    fn speech() {
        let cases = process(
            "alex: I am speaking hello there\n\
             alex: (Mood) I am speaking hello there\n\
             alex: I am speaking (Mood) hello there"
        );

        assert_eq!(cases[0], "<div class=\"name\">ALEX</div>\n<div class=\"speech\">I am speaking hello there</div>\n".to_string());
        assert_eq!(cases[1], "<div class=\"name\">ALEX</div>\n<div class=\"parens\">(Mood)</div>\n<div class=\"speech\">I am speaking hello there</div>\n".to_string());
        assert_eq!(cases[2], "<div class=\"name\">ALEX</div>\n<div class=\"speech\">I am speaking</div>\n\
                              <div class=\"parens\">(Mood)</div>\n<div class=\"speech\">hello there</div>\n".to_string());
    }
}

