use std::{fs,fmt};
use std::collections::HashMap;
use std::path::Path;
use atty::Stream;
use clap;
use exitcode;
use regex::Regex;
use serde::{Deserialize, Serialize};
use term;

// Still to implement:
//  * Command line interface (probably use `clap`)
//      - x Source only/certain output types only
//      - x Color/no color. Default to color unless stdout is redirected
//      - Count only (no matching)
//      - x Include cell number/cell execution count/line in cell
//      - x Case insensitive
//      - x Invert matching
//      - x With filename/without filename
//      - x Multiple files
//      - Recursive/include by glob pattern
//      - Maybe context lines/print whole cell?
//  * x Limiting to certain output types
//  * x Binary output match/no match
//  * Counting matches
//  * x Printing cell information
//  * x Case insensitivity
//  * x Iterating over multiple files
//  * Recursive searching

const TEXT_OUTPUT_DATA_TYPES: [&str;1] = ["text/plain"];
const DEFAULT_OUTPUTS: [&str;1] = ["text/plain"];

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
    invert_match: bool,
    show_line_detail: u8,
    show_file_name: bool
}

impl SearchOptions {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, RunErr> {
        let ignore_case = matches.occurrences_of("case") > 0;
        let invert_match = matches.occurrences_of("invert") > 0;

        let re = matches.value_of("pattern").unwrap();
        let re = if ignore_case {
            // In both cases the ?m (multi-line mode) flag is included
            // so that newlines at the end do not need to be included in
            // the regex to match with $ at the end. For example, the line
            // "Subsetting ci" will not match the regex "Subsetting [a-z]{2}$"
            // without the ?m flag because technically it is "Subsetting ci\n".
            format!("(?i)(?m){}", re)
        }else{
            format!("(?m){}", re)
        };

        let color = match matches.value_of("color").unwrap() {
            "always" => true,
            "never" => false,
            "auto" => atty::is(Stream::Stdout),
            _ => {return Err(RunErr::from("Unexpected value for '--color'"))}
        };

        // Because incl_src and no_incl_src override each other, and we want the default to be
        // include cell source text, we only need to check that there are no non-overridden
        // occurences of no_incl_src. Just checking "is_present" won't work - it's `true` even
        // if overridden.
        let n_skip_src = matches.occurrences_of("no_incl_src");
        let incl_src = n_skip_src == 0;

        // Which cell types we search. Default is all (markdown, raw, code)
        let cell_types = if let Some(vals) = matches.values_of("cell_types") {
            let mut tmp = Vec::new();
            for ct in vals {
                tmp.push(String::from(ct));
            }
            tmp
        }else{
            vec![String::from("markdown"), String::from("code"), String::from("raw")]
        };
        
        // Which output types to include
        let prelim_output_types = if let Some(vals) = matches.values_of("output_types") {
            let mut tmp = Vec::new();
            for ot in vals {
                tmp.push(String::from(ot));
            }
            tmp
        }else{
            to_string_vec(&DEFAULT_OUTPUTS)
        };

        let output_types = if matches.occurrences_of("incl_output") > 0 {
            to_string_vec(&DEFAULT_OUTPUTS)
        }else if matches.occurrences_of("no_incl_output") > 0 {
            Vec::new()
        }else{
            prelim_output_types
        };

        // Options controlling output detail
        let line_detail_level = if matches.occurrences_of("max_line_info") > 0 {
            255 as u8
        } else {
            matches.occurrences_of("line_info") as u8
        };
        let show_filenames_raw = matches.value_of("show_filenames").unwrap();
        let show_filenames = if matches.occurrences_of("force_show_file") > 0 {
            true
        } else if show_filenames_raw == "auto" {
            matches.occurrences_of("paths") > 1
        } else {
            show_filenames_raw == "always"
        };

        let opts = SearchOptions{
            re: Regex::new(&re)?,
            include_source: incl_src,
            include_cell_types: cell_types,//vec![String::from("markdown"), String::from("code")],
            include_output_types: output_types,
            color_matches: color,
            invert_match: invert_match,
            show_line_detail: line_detail_level,
            show_file_name: show_filenames
        };

        Ok(opts)
    }
}


struct MatchedLine<'a> {
    line: &'a str,
    line_number: usize,
    match_positions: Vec<(usize, usize)>,
    is_text: bool
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
            match_positions: self.match_positions.iter().cloned().collect(),
            is_text: self.is_text
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
    data: Option<HashMap<String, serde_json::Value>>, 
    text: Option<Vec<String>>,
    output_type: String
}

fn is_text(datatype: &str) -> bool {
    for &t in TEXT_OUTPUT_DATA_TYPES.iter() {
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


fn search_notebook(filename: &std::ffi::OsString, opts: &SearchOptions) -> Result<bool, RunErr> {
    let nb = load_notebook(filename)?;
    let mut found_match = false;

    for (icell, cell) in nb.cells.iter().enumerate() {
        if !opts.include_cell_types.contains(&cell.cell_type) {
            continue;
        }

        if opts.include_source {
            let lines = build_src_ref(&cell.source);
            let matches = search_text_lines(lines, opts);
            for m in matches {
                print_text_match(filename, &m, cell, icell, "source", opts);
                found_match = true;
            }
        }

        if let Some(outputs) = &cell.outputs {
            for outp in outputs {
                let matches = search_output(&outp, opts)?;
                // TODO: gracefully handle unexpected notebook format?
                for m in matches {
                    if m.is_text {
                        print_text_match(filename, &m, &cell, icell, "output/text", opts);
                    }else{
                        print_nontext_match(filename, &m, &cell, icell, "output/data", opts);
                    }
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

        let ml = MatchedLine{line: line, line_number: i, match_positions: inds, is_text: true};
        matched_lines.push(ml);
    }

    return matched_lines;
}

fn search_nontext_data<'a>(data: &'a str, opts: &SearchOptions) -> Option<MatchedLine<'a>> {
    if !opts.invert_match && !opts.re.is_match(data) {
        return None;
    }else if opts.invert_match && opts.re.is_match(data){
        return None;
    };

    Some(MatchedLine{line: data, line_number: 0, match_positions: Vec::new(), is_text: false})

}


fn search_output<'a>(outp: &'a Output, opts: &SearchOptions) -> Result<Vec<MatchedLine<'a>>, RunErr> {
    let mut matched_lines = Vec::new();

    if let Some(output_data) = &outp.data {
        for (dtype, val) in output_data.iter(){
            if !opts.include_output_types.contains(dtype) {
                // skip
            }else if is_text(dtype){
                let lines = convert_output_text_data(val)?;
                for m in search_text_lines(lines, opts) {
                    matched_lines.push(m);
                }
                
            }else{
                let data = convert_output_nontext_data(val)?;
                if let Some(m) = search_nontext_data(data, opts) {
                    matched_lines.push(m);
                }
            }
        }
    }

    if let Some(text_lines) = &outp.text {
        // This I think is the best way to do this. outp.text has to be a Vec<String>
        // because it holds the original instance of the strings read from the JSON file.
        // I tried making `search_text_lines` take a Vec<AsRef<str>> but didn't see a way
        // to indicate that the reference would stay valid long enough. This method 
        // creates refs that have lifetime 'a so we know they are okay to return from 
        // this function.
        let ref_lines: Vec<&str> = text_lines.iter().map(|x| x.as_ref()).collect();
        for m in search_text_lines(ref_lines, opts) {
            matched_lines.push(m);
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

fn convert_output_nontext_data<'a>(val: &'a serde_json::Value) -> Result<&'a str, RunErr> {
    let data = if let serde_json::Value::String(s) = val {
        s
    }else{
        return Err(RunErr::from("Unexpected type for nontext data"));
    };

    Ok(data)
}


fn print_line_detail(file_name: &std::ffi::OsString, m: &MatchedLine, cell: &Cell, icell: usize, cell_piece: &str, opts: &SearchOptions) {
    if opts.show_file_name {
        print!("{:?}: ", file_name);
    }
    if opts.show_line_detail == 0 {
        print!("\t");
        return
    }

    let exec_cnt_str = if let Some(n) = cell.execution_count {
        format!(" [{}]", n)
    }else{
        if opts.show_line_detail < 4 {String::from("")}
        else {String::from("[None]")}
    };

    let info = match opts.show_line_detail {
        1 => format!("c.{} l.{}", icell, m.line_number+1),
        2 => format!("c.{}{} l.{}", icell, exec_cnt_str, m.line_number+1),
        3 => format!("c.{}{} ({}) l.{}", icell, exec_cnt_str, cell_piece, m.line_number+1),
        _ => format!("Cell #{} (exec. {}) {}, line {}", icell, exec_cnt_str, cell_piece, m.line_number+1)
    };

    print!("{}: \t", info);
}


fn print_text_match(filename: &std::ffi::OsString, m: &MatchedLine, cell: &Cell, icell: usize, cell_piece: &str, opts: &SearchOptions) {
    // Print the line - if not coloring matches, then we can just print it,
    // otherwise we have to iterate over the matches and switch to colored/bolded. How to color:
    // https://mmstick.gitbooks.io/rust-programming-phoronix-reader-how-to/content/chapter11.html
    print_line_detail(filename, m, cell, icell, cell_piece, opts);

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

                        color_on(&mut terminal);
                        //terminal.fg(term::color::BRIGHT_RED).unwrap();
                        //terminal.attr(term::Attr::Bold).unwrap();
                    }else if m.at_any_match_stop(idx) {
                        let s = String::from_utf8(curr_bytes.clone()).unwrap();
                        print!("{}", s);
                        curr_bytes.clear();
                        curr_bytes.push(b);

                        color_off(&mut terminal);
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


fn print_nontext_match(filename: &std::ffi::OsString, m: &MatchedLine, cell: &Cell, icell: usize, cell_piece: &str, opts: &SearchOptions) {
    print_line_detail(filename, m, cell, icell, cell_piece, opts);
    print_colored("Non-text output data matches.");
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

fn to_string_vec(a: &[&str]) -> Vec<String> {
    let mut tmp = Vec::new();
    for &el in a {
        tmp.push(String::from(el));
    }
    tmp
}

fn print_colored(msg: &str) {
    let termopt = term::stdout();
    match termopt {
        None => {print!("{}", msg)},
        Some(mut terminal) => {
            color_on(&mut terminal);
            print!("{}", msg);
            color_off(&mut terminal);
        }
    }
}

fn color_on(terminal: &mut std::boxed::Box<dyn term::Terminal<Output = std::io::Stdout> + std::marker::Send>) {
    terminal.fg(term::color::BRIGHT_RED).unwrap();
    terminal.attr(term::Attr::Bold).unwrap();
}

fn color_off(terminal: &mut std::boxed::Box<dyn term::Terminal<Output = std::io::Stdout> + std::marker::Send>) {
    terminal.reset().unwrap();
}


fn get_notebooks_in_dir(dirpath: &Path, file_list: &mut Vec<std::ffi::OsString>, recurse: bool) -> Result<(), RunErr> {
    for entry in dirpath.read_dir()? {
        if let Ok(entry) = entry {
            let entry_path = entry.path();
            if entry_path.is_dir() && recurse {
                get_notebooks_in_dir(&entry_path, file_list, recurse)?;
            }else if entry_path.is_file() {
                if let Some(ext) = entry_path.extension() {
                    if ext == "ipynb" {
                        file_list.push(std::ffi::OsString::from(entry_path))
                    }
                }
            }
        }
    }

    Ok(())
}


fn parse_clargs() -> Result<(Vec<std::ffi::OsString>, SearchOptions), RunErr> {
    let yml = clap::load_yaml!("clargs.yml");
    let clargs = clap::App::from_yaml(yml).version(clap::crate_version!()).get_matches();

    let paths_raw = clargs.values_of_os("paths").unwrap();
    let mut paths: Vec<std::ffi::OsString> = Vec::new();
    for p in paths_raw {
        let curr_path = Path::new(p);
        if curr_path.is_file() {
            paths.push(std::ffi::OsString::from(p));
        }else if curr_path.is_dir() {
            get_notebooks_in_dir(curr_path, &mut paths, false)?;
        } 
    }

    if paths.len() == 0 {
        return Err(RunErr{msg: "No notebook files listed or found in the given directories.".to_string()})
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
        match search_notebook(&filename, &opts) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Error in file {:?}: {}", &filename, e);
                continue;
            }
        };
    }
}
