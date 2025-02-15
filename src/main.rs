use std::{
    env,
    io::Write,
    process::ExitCode,
};
use scripts::{ CmdInfo, Command };


const VERSION: &str = env!("CARGO_PKG_VERSION");


fn get_command(args: &[String]) -> Result<Command, String> {
    let input = args::parser!{
        ["--version"+],
        ["--help"+],
        ["-i",        String],
        ["-o",        String],
        ["--temp"],
        ["--nopen"],
        ["--scenes"+, String]
    }.parse_manual(args);

    if input.has("--version") {
        return Ok(Command::Version)
    }
    if input.has("--help") {
        return Ok(Command::Help)
    }

    let mut cmd: CmdInfo = CmdInfo::default();

    if let Some(Some(i)) = input.get("-i") { // Some(arg Some(param))
        cmd.infile = i.as_string().unwrap().to_owned();
    } else {
        return Err("ERROR: input file not provided".into())
    }
    if let Some(Some(o)) = input.get("-o") {
        cmd.outfile = o.as_string().unwrap().to_owned();
    } else {
        return Err("ERROR: output file not provided".into())
    }
    cmd.temp  = input.has("--temp");
    cmd.nopen = input.has("--nopen");

    if let Some(Some(s)) = input.get("--scenes") {
        let range = s.as_string().unwrap();

        if let Some(j) = range.find('-') {
            let start: u32 = range[0..j].parse().map_err(|_| "ERROR: range argument was not integer".to_string())?;
            let stop: u32 = range[(j+1)..range.len()].parse().map_err(|_| "ERROR: range argument was not integer".to_string())?;
            cmd.range = Some(start..(stop+1));
        } else {
            let start: u32 = range.parse().map_err(|_| "ERROR: scene argument was not integer".to_string())?;
            cmd.range = Some(start..(start+1));
        }
    }

    cmd.file_root = cmd.infile.strip_suffix(".txt").ok_or("ERROR: expected '.txt' file as input")?.to_string();
    cmd.exe_loc = env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();

    cmd.html = format!("{}/../../user/temp.html", cmd.exe_loc);

    Ok(Command::Convert(cmd))
}

fn cmd_help() -> ExitCode {
    cmd_version();
    println!(r#"
Synopsis:
    scripts [OPTIONS] -i <input file> -o <output file>

Options:
    -i <path to source>     Path to input '.txt' file, formatted in provided specification
    -o <path to output>     Path to output '.pdf' file
        --temp              Include intermediate html in output
    -s, --scenes <range>    Output selected scenes without title page
    -v, --version           Show version information
    -h, --help              Show documentation

Format guide:
    scene   [CONTENT]               Begin new scene
    trans   [CONTENT]               Transition annotation
    direct  [CONTENT]               Action lines
    subhead [CONTENT]               Subheading
    chyron  [CONTENT]               Title or text
    parens  [CONTENT]               Parenthetical
    speech  [CONTENT]               Character speech
    montage                         Begin scene montage
    mon-end                         End scene montage
    [NAME]: [CONTENT]               Named character speech
    [NAME]: ([PARENS]) [CONTENT]    Named character speech with parenthetical
    *                               Inline comment
    ***                             File tail comment

Notes:
    Title and subtitle MUST be provided in any 2 lines before regular content
    Any segment may be continued on a new line using a backslash '\' character
    Empty lines may be placed anywhere for readability, as they will be ignored"#);

    0.into()
}

fn cmd_version() -> ExitCode {
    println!(r#"Screenplay to PDF converter
    -Alex Vance
    -Version {VERSION}"
    -Built with Rust 2021"#);

    0.into()
}

fn cmd_convert(cmd: CmdInfo) -> ExitCode {
    print!("Generating html...\t");

    if let Err(err) = scripts::gen_html(&cmd) {
        let _ = std::io::stdout().flush();
        eprintln!("ERROR: falied to generate html: {err}");
        return 2.into();
    }

    println!("complete");
    println!("Invoking webkit:\n");

    match  scripts::gen_pdf(&cmd) {
        Err(err) => {
            eprintln!("ERROR: falied to invoke webkit: {err}");
            return 3.into()
        }
        Ok(code) => if code.success() {
            println!("\nConversion completed successfully");
        } else {
            eprintln!("ERROR: falied to generate pdf: {code}");
            return 4.into()
        }
    }

    if !cmd.nopen {
        if let Err(err) = open::that(cmd.outfile) {
            eprintln!("ERROR: falied to open pdf in default app: {err}");
            return 5.into()
        }
    }

    0.into()
}


fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().collect();

    if args.len() == 1 {
        eprintln!("ERROR: no arguments found");
        return cmd_help()
    }

    match get_command(&args) {
        Ok(cmd) =>{
            match cmd {
                Command::Help => cmd_help(),
                Command::Version => cmd_version(),
                Command::Convert(c) => cmd_convert(c),
            }
        }
        Err(err) => {
            eprintln!("{err}");
            1.into()
        }
    }
}

