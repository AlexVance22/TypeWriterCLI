use std::env;
use std::process::ExitCode;
use std::collections::HashSet;
use scripts::{ CmdInfo, Query, Command };


const VERSION: &str = env!("CARGO_PKG_VERSION");


fn get_command(args: &[String]) -> Result<Command, String> {
    let valid: HashSet<&str> = ["-v", "--version", "-h", "--help", "--temp", "--nopen", "-i", "-o", "-s", "--scenes"].into_iter().collect();
    for a in args {
        if a.starts_with('-') && !valid.contains(a.as_str()) {
            return Err(format!("ERROR: invalid option specified: {a}"))
        }
    }

    if args.contains(&"-h".to_string()) || args.contains(&"--help".to_string()) {
        return Ok(Command::Help)
    }
    if args.contains(&"-v".to_string()) || args.contains(&"--version".to_string()) {
        return Ok(Command::Version)
    }

    let mut cmd: CmdInfo = CmdInfo::default();

    if let Some(i) = args.iter().position(|s| s == "-i") {
        cmd.infile = args.get(i + 1).ok_or("ERROR: input file not provided")?.clone();
    } else {
        return Err("ERROR: input file not provided".to_string())
    }
    if let Some(i) = args.iter().position(|s| s == "-o") {
        cmd.outfile = args.get(i + 1).ok_or("ERROR: output file not provided")?.clone();
    } else {
        return Err("ERROR: output file not provided".to_string())
    }

    if args.contains(&"--temp".to_string()) {
        cmd.temp = true;
    }

    if args.contains(&"--nopen".to_string()) {
        cmd.nopen = true;
    }

    if let Some(i) = args.iter().position(|s| s == "-s" || s == "--scenes") {
        let range = args.get(i + 1).ok_or("ERROR: scene spec declared but not provided")?.clone();

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

    cmd.html = format!("{}/user/temp.html", cmd.exe_loc);

    Ok(Command::Convert(cmd))
}


fn cmd_help() -> ExitCode {
    cmd_version();
    println!();
    println!("Synopsis:");
    println!("  scripts [OPTIONS] -i <input file> -o <output file>");
    println!();
    println!("Options:");
    println!("  -i <path to source>     Path to input '.txt' file, formatted in provided specification");
    println!("  -o <path to output>     Path to output '.pdf' file");
    println!("      --temp              Include intermediate html in output");
    println!("  -s, --scenes <range>    Output selected scenes without title page");
    println!("  -v, --version           Show version information");
    println!("  -h, --help              Show documentation");
    println!();
    println!("Format guide:");
    println!("  scene   [CONTENT]               Begin new scene");
    println!("  trans   [CONTENT]               Transition annotation");
    println!("  direct  [CONTENT]               Action lines");
    println!("  subhead [CONTENT]               Subheading");
    println!("  chyron  [CONTENT]               Title or text");
    println!("  parens  [CONTENT]               Parenthetical");
    println!("  speech  [CONTENT]               Character speech");
    println!("  montage                         Begin scene montage");
    println!("  mon-end                         End scene montage");
    println!("  [NAME]: [CONTENT]               Named character speech");
    println!("  [NAME]: ([PARENS]) [CONTENT]    Named character speech with parenthetical");
    println!("  *                               Inline notes section");
    println!("  ***                             End (notes section)");

    0.into()
}

fn cmd_version() -> ExitCode {
    println!("Screenplay to PDF converter");
    println!("-Alex Vance");
    println!("-Version {VERSION}");
    println!("-Built with Rust 2021");

    0.into()
}

fn cmd_convert(cmd: CmdInfo) -> ExitCode {
    print!("Generating html...\t");

    if let Err(err) = scripts::gen_html(&cmd) {
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

fn cmd_status(query: Query) -> ExitCode {
    if let Err(err) = scripts::get_status(query) {
        eprintln!("ERROR: falied to fetch requested info: {err}");
        return 1.into();
    }

    0.into()
}


fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

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
                Command::Status(c) => cmd_status(c),
            }
        }
        Err(err) => {
            eprintln!("{err}");
            1.into()
        }
    }
}
