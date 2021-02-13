use std::fs;
use std::collections::HashMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

const TEXT_OUTPUT_TYPES: [&str;1] = ["text/plain"];

#[derive(Debug)]
struct RunErr {
    msg: String
}

impl From<std::io::Error> for RunErr {
    fn from(error: std::io::Error) -> Self {
        let msg = error.to_string();
        Self{msg}
    }
}

impl From<serde_json::Error> for RunErr {
    fn from(error: serde_json::Error) -> Self {
        let msg = error.to_string();
        Self{msg}
    }
}

impl From<&str> for RunErr {
    fn from(msg: &str) -> Self {
        Self{msg: String::from(msg)}
    }
}


struct SearchOptions {
    re: Regex,
    include_source: bool,
    include_cell_types: Vec<String>,
    include_output_types: Vec<String>
}


struct MatchedLine<'a> {
    line: &'a str,
    line_number: usize,
    match_positions: Vec<(usize, usize)>
}

impl Clone for MatchedLine<'_> {
    fn clone(&self) -> Self{
        Self{
            line: self.line,
            line_number: self.line_number,
            match_positions: self.match_positions.iter().cloned().collect()
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Notebook {
    cells: Vec<Cell>
}

#[derive(Serialize, Deserialize)]
struct Cell {
    cell_type: String,
    execution_count: Option<usize>,
    source: Vec<String>,
    outputs: Option<Vec<Output>>
}

#[derive(Serialize, Deserialize, Debug)]
struct Output {
    // data must be a hash map of Value enums because some outputs are arrays ("text/plain")
    // and others are just a string ("image/png"). Would've just made a structure for
    // the output data with each type but (a) that's not very extensible and (b) can't have
    // slashes in field names 
    data: HashMap<String, serde_json::Value>, 
    output_type: String
}


fn is_text(datatype: &str) -> bool {
    for &t in TEXT_OUTPUT_TYPES.iter() {
        if t == datatype {
            return true;
        }
    }

    return false;
}


fn load_notebook(path: &str) -> Result<Notebook, RunErr>{
    let data = fs::read_to_string(path)?;
    let notebook: Notebook = serde_json::from_str(&data)?;

    Ok(notebook)
}


fn search_notebook(nb: &Notebook, opts: &SearchOptions) -> Result<bool, RunErr> {
    let mut found_match = false;

    for cell in nb.cells.iter() {
        if !opts.include_cell_types.contains(&cell.cell_type) {
            continue;
        }

        if opts.include_source {
            let lines = build_src_ref(&cell.source);
            let matches = search_text_lines(lines, opts);
            for m in matches {
                print_text_match(&m, cell, opts);
                found_match = true;
            }
        }

        if let Some(outputs) = &cell.outputs {
            for outp in outputs {
                
            }
        }
    }

    Ok(found_match)
}

fn build_src_ref(source: &Vec<String>) -> Vec<&str> {
    let mut v = Vec::with_capacity(source.len());
    for el in source.iter() {
        v.push(el.as_ref());
    }
    return v;
}


fn search_text_lines<'a>(text: Vec<&'a str>, opts: &SearchOptions) -> Vec<MatchedLine<'a>> {
    let mut matched_lines: Vec<MatchedLine> = Vec::new();
    for (i, line) in text.iter().enumerate() {
        if !opts.re.is_match(line.as_ref()) {
            continue;
        }

        let mut inds = Vec::new();
        for m in opts.re.find_iter(line.as_ref()) {
            inds.push((m.start(), m.end()));
        }

        let ml = MatchedLine{line: line, line_number: i, match_positions: inds};
        matched_lines.push(ml);
    }

    return matched_lines;
}


fn search_output<'a>(outp: &'a Output, opts: &SearchOptions) -> Result<Vec<MatchedLine<'a>>, RunErr> {
    let mut matched_lines = Vec::new();

    for (dtype, val) in outp.data.iter(){
        if is_text(dtype){
            let lines = convert_output_text_data(val)?;
            for m in search_text_lines(lines, opts) {
                matched_lines.push(m.clone());
            }
            
        }else{

        }
    }

    // I see the problem: the matches reference the lines created by `convert_output_text_data` which go
    // out of scope. Either I need to just make MatcheLine hold a copy String or tell Rust that the underlying
    // string value is still in memory.
    return Ok(matched_lines);
}

fn convert_output_text_data<'a>(val: &'a serde_json::Value) -> Result<Vec<&'a str>, RunErr> {
    let arr = if let serde_json::Value::Array(a) = val {
        a
    }else{
        return Err(RunErr::from("Expected an array for output text values."));
    };
    let mut text_lines: Vec<&str> = Vec::with_capacity(arr.len());

    for el in arr.iter() {
        if let serde_json::Value::String(s) = el {
            text_lines.push(s);
        }else{
            return Err(RunErr::from("Expected a string for all elements of output text value"));
        }
    }

    Ok(text_lines)
}

fn print_text_match(m: &MatchedLine, cell: &Cell, opts: &SearchOptions) {

}


fn main() {
    let nb = load_notebook("example-notebooks/demo.ipynb").unwrap();
    println!("Notebook has {} cells", nb.cells.len());
    println!("Cell 0 source = {:#?}", nb.cells[0].source);

    let out = &nb.cells[4].outputs.as_ref().unwrap();
    println!("Cell 4 outputs[0] = {:#?}", &out[0]);

    let opts = SearchOptions{
        re: Regex::new(r"gpd").unwrap(),
        include_source: true,
        include_cell_types: vec![String::from("markdown"), String::from("code")],
        include_output_types: vec![String::from("text/plain")]
    };
    search_notebook(&nb, &opts);
}
