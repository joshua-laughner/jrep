use std::{fs,fmt};
use std::collections::HashMap;
use atty::Stream;
use clap;
use exitcode;
use regex::Regex;
use serde::{Deserialize, Serialize};
use term;

// Still to implement:
//  * Command line interface (probably use `clap`)
//      - Source only/certain output types only
//      - x Color/no color. Default to color unless stdout is redirected
//      - Count only (no matching)
//      - Include cell number/cell execution count/line in cell
//      - x Case insensitive
//      - x Invert matching
//      - With filename/without filename
//      - x Multiple files
//      - Recursive/include by glob pattern
//      - Maybe context lines/print whole cell?
//  * Limiting to certain output types
//  * Binary output match/no match
//  * Counting matches
//  * Printing cell information
//  * x Case insensitivity
//  * x Iterating over multiple files
//  * Recursive searching

const TEXT_OUTPUT_TYPES: [&str;1] = ["text/plain"];

#[derive(Debug)]
struct RunErr {
    msg: String
}

impl fmt::Display for RunErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)?;
        fmt::Result::Ok(())
    }
}

impl From<std::io::Error> for RunErr {
    fn from(error: std::io::Error) -> Self {
        let msg = error.to_string();
        Self{msg}
    }
}

impl From<regex::Error> for RunErr {
    fn from(error: regex::Error) -> Self {
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
    include_output_types: Vec<String>,
    color_matches: bool,
    invert_match: bool
}

impl SearchOptions {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, RunErr> {
        let ignore_case = matches.occurrences_of("case") > 0;
        let invert_match = matches.occurrences_of("invert") > 0;

        let re = matches.value_of("pattern").unwrap();
        let re = if ignore_case {
            format!("(?i){}", re)
        }else{
            String::from(re)
        };

        let color = match matches.value_of("color").unwrap() {
            "always" => true,
            "never" => false,
            "auto" => atty::is(Stream::Stdout),
            _ => {return Err(RunErr::from("Unexpected value for '--color'"))}
        };

        let opts = SearchOptions{
            re: Regex::new(&re)?,
            include_source: true,
            include_cell_types: vec![String::from("markdown"), String::from("code")],
            include_output_types: vec![String::from("text/plain")],
            color_matches: color,
            invert_match
        };

        Ok(opts)
    }
}


struct MatchedLine<'a> {
    line: &'a str,
    line_number: usize,
    match_positions: Vec<(usize, usize)>
}

impl MatchedLine<'_> {
    fn at_any_match_start(&self, idx: usize) -> bool {
        for &(start, _stop) in self.match_positions.iter() {
            if start == idx {
                return true;
            }
        }

        return false;
    }

    fn at_any_match_stop(&self, idx: usize) -> bool {
        for &(_start, stop) in self.match_positions.iter() {
            if stop == idx {
                return true;
            }
        }

        return false;
    }
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


fn load_notebook(path: &std::ffi::OsString) -> Result<Notebook, RunErr>{
    let data = fs::read_to_string(path)?;
    let notebook: Notebook = serde_json::from_str(&data)?;

    Ok(notebook)
}


fn search_notebook(nb: &Notebook, opts: &SearchOptions) -> Result<bool, RunErr> {
    let mut found_match = false;

    for (icell, cell) in nb.cells.iter().enumerate() {
        if !opts.include_cell_types.contains(&cell.cell_type) {
            continue;
        }

        if opts.include_source {
            let lines = build_src_ref(&cell.source);
            let matches = search_text_lines(lines, opts);
            for m in matches {
                print_text_match(&m, cell, &icell, opts);
                found_match = true;
            }
        }

        if let Some(outputs) = &cell.outputs {
            for outp in outputs {
                let matches = search_output(&outp, opts)?;
                // TODO: gracefully handle unexpected notebook format?
                for m in matches {
                    print_text_match(&m, &cell, &icell, opts);
                    found_match = true;
                }
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
        if !opts.invert_match && !opts.re.is_match(line.as_ref()) {
            continue;
        }else if opts.invert_match && opts.re.is_match(line.as_ref()) {
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

fn print_text_match(m: &MatchedLine, cell: &Cell, icell: &usize, opts: &SearchOptions) {
    // Print the line - if not coloring matches, then we can just print it,
    // otherwise we have to iterate over the matches and switch to colored/bolded. How to color:
    // https://mmstick.gitbooks.io/rust-programming-phoronix-reader-how-to/content/chapter11.html

    if !opts.color_matches {
        let mut s = String::from(m.line);
        trim_newline(&mut s);
        print!("{}", s);
    }else{
        let termopt = term::stdout();
        match termopt {
            None => {print!("{}", m.line)},
            Some(mut terminal) => {
                let mut curr_bytes: Vec<u8> = Vec::new();
                for (idx, b) in m.line.bytes().enumerate()  {
                    // The start/end values from the regex are byte offsets: https://docs.rs/regex/1.4.3/regex/struct.Match.html
                    // Since strings are unicode encoded, we'll probably need to iterate over bytes until we hit one of the 
                    // match start or end indices, then convert back to unicode (if possible - if not, print raw bytes? ASCII?),
                    // print, and switch the terminal to either colored & bolded or reset.
                    if m.at_any_match_start(idx) {
                        // TODO: gracefully handle failed UTF conversion (if match ends in middle of a unicode character)
                        let s = String::from_utf8(curr_bytes.clone()).unwrap();
                        print!("{}", s);
                        curr_bytes.clear();
                        curr_bytes.push(b);

                        terminal.fg(term::color::BRIGHT_RED).unwrap();
                        terminal.attr(term::Attr::Bold).unwrap();
                    }else if m.at_any_match_stop(idx) {
                        let s = String::from_utf8(curr_bytes.clone()).unwrap();
                        print!("{}", s);
                        curr_bytes.clear();
                        curr_bytes.push(b);

                        terminal.reset().unwrap();
                    }else{
                        curr_bytes.push(b);
                    }
                }

                // There should always be at least one character left since the match stop index is exclusive
                // (if the match goes to the end of the line, then `at_any_match_stop` will still be false at 
                // the last byte's index). Also no need to clone - last time we'll use this
                let mut s = String::from_utf8(curr_bytes).unwrap();
                trim_newline(&mut s);
                print!("{}", s);
                terminal.reset().unwrap();
            }
        }
    }
    
    println!();
}


fn trim_newline(s: &mut String) {
    // https://stackoverflow.com/a/55041833
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}


fn parse_clargs() -> Result<(Vec<std::ffi::OsString>, SearchOptions), RunErr> {
    let yml = clap::load_yaml!("clargs.yml");
    let clargs = clap::App::from_yaml(yml).version(clap::crate_version!()).get_matches();

    let paths_raw = clargs.values_of_os("paths").unwrap();
    let mut paths: Vec<std::ffi::OsString> = Vec::new();
    for p in paths_raw {
        paths.push(std::ffi::OsString::from(p));
    }

    let opts = match SearchOptions::from_arg_matches(&clargs){
        Ok(o) => o,
        Err(e) => {
            let msg = format!("The search pattern was not valid: {}", e);
            return Err(RunErr{msg})
        }
    };
    return Ok((paths, opts));
}


fn main() {
    let (paths, opts) = match parse_clargs() {
        Ok((p,o)) => (p,o),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(exitcode::USAGE);
        }
    };

    for filename in paths {
        let nb = match load_notebook(&filename) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("Error reading file {:?}: {}", &filename, e);
                continue;
            }
        };
        match search_notebook(&nb, &opts) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Error searching file {:?}: {}", &filename, e);
                continue;
            }
        };
    }
}
