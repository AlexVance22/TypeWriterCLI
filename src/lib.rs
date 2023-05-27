mod html;
mod pdf;

use std::ops::Range;
pub use html::*;
pub use pdf::*;


#[derive(Debug, Default, Clone)]
pub struct CmdInfo {
    pub infile: String,
    pub outfile: String,
    pub html: String,

    pub file_root: String,
    pub exe_loc: String,

    pub range: Option<Range<u32>>,
    pub temp: bool,
    pub nopen: bool,
}


#[derive(Debug, Clone)]
pub enum Command {
    Help,
    Version,
    Convert(CmdInfo),
}

