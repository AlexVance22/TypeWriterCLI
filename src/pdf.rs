use std::process::{ Command, ExitStatus };


pub fn gen_pdf(cmd: &super::CmdInfo) -> std::io::Result<ExitStatus> {
    let program = format!("{}/../../wkhtmltopdf.exe", cmd.exe_loc);

    Command::new(program)
        .args(["--margin-top", "1in"])
        .args(["--margin-bottom", "1in"])
        .args(["--margin-left", "0in"])
        .args(["--margin-right", "0in"])
        .arg("--grayscale")
        .arg(&cmd.html)
        .args(["--encoding", "utf-8"])
        //.args(["--user-style-sheet", "\"../res/style.css\""])
        .arg("--disable-smart-shrinking")
        .arg("--enable-local-file-access")
        .arg(&cmd.outfile)
        .status()
}

