use crate::Query;
use crate::html::{ Segments };
use std::fs;


pub fn get_status(query: Query) -> Result<(), String> {
    let content = fs::read_to_string(&query.infile).unwrap();
    let mut segments = Segments::new(&content);

    segments.next();
    segments.next();

    for (line, seg) in segments {
        
    }

    Ok(())
}

