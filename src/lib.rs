mod html;
mod pdf;
mod status;

use std::ops::Range;
pub use html::*;
pub use pdf::*;
pub use status::*;


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

#[derive(Debug, Default, Clone)]
pub struct Query {
    pub infile: String,
    pub file_root: String,
    pub exe_loc: String,
}


#[derive(Debug, Clone)]
pub enum Command {
    Help,
    Version,
    Convert(CmdInfo),
    Status(Query),
}

