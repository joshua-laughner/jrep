//! Search Jupyter notebooks from the command line
//! 
//! ## Why `jrep` exists
//! Fundamentally, Jupyter notebooks are just JSON files, so if you wanted
//! to quickly search for a string across many notebooks, you could use a
//! program like `grep`. But, when figures or other images are captured as
//! output, data will be a long jumble of encoded data which will (a) possibly
//! match your search expression and (b) be such a long "line" of text that it
//! completely swamps the rest of the (actually useful) grep output.
//! 
//! `jrep` is built to first parse Jupyter notebooks and only then search certain
//! types of cells. This way, it returns relevant results rather than messes of
//! image data.
//! 
//! ## Quick examples
//! 
//! Search cell inputs and plain text outputs in all notebooks in the current directory
//! for the string "numpy":
//! 
//! ```bash
//! jrep numpy
//! ```
//! 
//! Search just the notebook `demo.ipynb` for the string "numpy":
//! 
//! ```bash
//! jrep numpy demo.ipynb
//! ```
//! 
//! Search all the notebooks in the directory `~/Documents/Notebooks` for the string "numpy":
//! 
//! ```bash
//! jrep numpy ~/Documents/Notebooks
//! ```
//! 
//! Search only markdown cells in `demo.ipynb` for web URLs starting with "http://" or "https://"
//! (using regular expressions to indicate that the "s" is optional):
//! 
//! ```bash
//! jrep --cell-type markdown 'https?://' demo.ipynb
//! ```
//! 
//! ## Which notebooks are searched
//! 
//! Since the main use case is finding notebooks that contain a certain string, `jrep` will
//! search all notebooks (i.e. files with the `.ipynb` extension) in the current directory if
//! you do not explicitly specify paths to search as the second and later positional arguments.
//! 
//! You can specify exactly which notebook(s) to search by including them as arguments on the command
//! line. The following would search only the notebooks `demo.ipynb` and `example.ipynb`, not any other
//! notebooks, for the string "import":
//! 
//! ```bash
//! jrep import demo.ipynb example.ipynb
//! ```
//! 
//! Alternatively, you can specify directories as arguments, and any `*.ipynb` files in those directories
//! will be searched. By default, `jrep` will *not* recurse into other directories; to enable that behavior,
//! use the `--recursive` (short form: `-R`) flag. The following would search all notebooks in `~/Notebooks`
//! for "import":
//! 
//! ```bash
//! jrep import ~/Notebooks
//! ```
//! 
//! You can mix and match directories and notebooks in the arguments, e.g.:
//! 
//! ```bash
//! jrep import demo.ipynb ~/Notebooks
//! ```
//! 
//! Note however, that when searching a directory, *only* files ending in `.ipynb` are searched. Currently
//! there is no option to search other file extensions.
//! 
//! ## Understanding which cells are searched
//! 
//! At the top-most level, Jupyter notebooks consist of cells. Each cell has 
//! source data (the text or code that you enter) and may have zero, one, or
//! multiple output elements. Each cell is classified as "code", "markdown",
//! or "raw" based on the type of source data it has. The output elements
//! have a wider variety of types, but some common ones include "text/plain"
//! and "image/png".
//! 
//! `jrep` will, by default, search the source data for all cell types, but
//! only output from code cells with the type "text/plain". This gives you default
//! behavior to search all or most human-readable text in the notebooks, but not
//! any image or other non-text output. You can change this behavior with several
//! of the command line flags:
//! 
//! * To limit which cell types (i.e. markdown, raw, or code) are searched, use the
//!   `--cell-types` (short form: `-t`) option. You can specify this more than once
//!   if you want to search two of the cell types, e.g. `-t markdown -t raw`.
//! * To turn off searching the source data (i.e. input) of the cells, use
//!   `--no-include-source` (short form: `-X`). This is just a flag, it doesn't take
//!   any arguments.
//! * To turn off searching outputs from code cells, use `--no-include-output`.
//! * To change which output types are searched, use `--output-type` (short form: `-O`)
//!   followed by the type. For example, if you did want to search image output (for some
//!   reason), you could use `-O image/png`. Note that specifying any `--output-type` options
//!   overrides the default of "text/plain". That means that if your notebook has outputs
//!   of both "text/plain" and "text/latex" that you want to search, you need to pass both
//!   types as options, i.e. `-O text/plain -O text/latex`.
//! 
//! ## Specifying the search string
//! 
//! `jrep` treats the search pattern given to it as a regular expression. This means that both
//! searches for simple patterns, such as "numpy" or "import", and more abstract patterns, such
//! as "foo\s?=\s?[a-zA-Z]" can be used. The regular expression syntax should be generally
//! similar to that of [grep](https://www.man7.org/linux/man-pages/man1/grep.1.html), although
//! this is not strictly enforced. `jrep` uses the [Regex crate](https://docs.rs/regex/latest/regex/),
//! so see their [syntax page](https://docs.rs/regex/latest/regex/#syntax) for the exact syntax
//! supported.
//! 
//! Note that your shell may interpret certain special characters in the regular expressions itself -
//! especially `*`, `?`, `{`, `}`, and `\`. If you're giving a regular expression as the pattern for
//! `jrep` to search for, you will probably have the best luck if you wrap it in single quotes (e.g.
//! `jrep 'foo\s?=\s?[a-zA-Z]'`), but your experience may depend on what shell you use.
//! 
//! The default behavior is for `jrep` to respect the case of the search string, meaning the pattern
//! "Foo" will not match "foo" in the notebooks. You can set `jrep` to ignore case with the `--ignore-case`
//! (short form: `-i`) flag.
//! 
//! ## The rest of the interface
//! 
//! There are many more command line options not described here. They are all explained in the command line
//! interface itself, and can be viewed with `jrep --help`. 



use std::{fs,fmt};
use std::collections::{HashMap,HashSet};
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
//  * Alternate mode that prints out the type of each cell and of each output, so that users
//    can figure out what output types they have more easily.

#[doc(hidden)]
const TEXT_OUTPUT_DATA_TYPES: [&str;1] = ["text/plain"];
#[doc(hidden)]
const DEFAULT_OUTPUTS: [&str;1] = ["text/plain"];

#[derive(Debug)]
#[doc(hidden)]
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


#[doc(hidden)]
struct SearchOptions {
    re: Regex,
    include_source: bool,
    include_cell_types: Vec<String>,
    include_output_types: Vec<String>,
    color_matches: bool,
    invert_match: bool,
    show_line_detail: u8,
    show_file_name: bool,
    recursive: bool
}

impl SearchOptions {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, RunErr> {
        let ignore_case = matches.occurrences_of("case") > 0;
        let invert_match = matches.occurrences_of("invert") > 0;
        let recursive = matches.occurrences_of("recursive") > 0;

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
            let mut paths_raw = matches.values_of_os("paths").unwrap();
            // Assume that if one of the input paths is a directory that
            // we should print the file names so that we know which file
            // is matching.
            matches.occurrences_of("paths") > 1 || paths_raw.any(|x| Path::new(x).is_dir())
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
            show_file_name: show_filenames,
            recursive: recursive
        };

        Ok(opts)
    }
}


#[doc(hidden)]
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
#[doc(hidden)]
struct Notebook {
    cells: Vec<Cell>
}

#[derive(Serialize, Deserialize)]
#[doc(hidden)]
struct Cell {
    cell_type: String,
    execution_count: Option<usize>,
    source: Vec<String>,
    outputs: Option<Vec<Output>>
}

#[derive(Serialize, Deserialize, Debug)]
#[doc(hidden)]
struct Output {
    // data must be a hash map of Value enums because some outputs are arrays ("text/plain")
    // and others are just a string ("image/png"). Would've just made a structure for
    // the output data with each type but (a) that's not very extensible and (b) can't have
    // slashes in field names 
    data: Option<HashMap<String, serde_json::Value>>, 
    text: Option<Vec<String>>,
    output_type: String
}

#[doc(hidden)]
fn is_text(datatype: &str) -> bool {
    for &t in TEXT_OUTPUT_DATA_TYPES.iter() {
        if t == datatype {
            return true;
        }
    }

    return false;
}


#[doc(hidden)]
fn load_notebook(path: &std::ffi::OsString) -> Result<Notebook, RunErr>{
    let data = fs::read_to_string(path)?;
    let notebook: Notebook = serde_json::from_str(&data)?;

    Ok(notebook)
}


#[doc(hidden)]
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

#[doc(hidden)]
fn build_src_ref(source: &Vec<String>) -> Vec<&str> {
    let mut v = Vec::with_capacity(source.len());
    for el in source.iter() {
        v.push(el.as_ref());
    }
    return v;
}


#[doc(hidden)]
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

#[doc(hidden)]
fn search_nontext_data<'a>(data: &'a str, opts: &SearchOptions) -> Option<MatchedLine<'a>> {
    if !opts.invert_match && !opts.re.is_match(data) {
        return None;
    }else if opts.invert_match && opts.re.is_match(data){
        return None;
    };

    Some(MatchedLine{line: data, line_number: 0, match_positions: Vec::new(), is_text: false})

}


#[doc(hidden)]
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

#[doc(hidden)]
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

#[doc(hidden)]
fn convert_output_nontext_data<'a>(val: &'a serde_json::Value) -> Result<&'a str, RunErr> {
    let data = if let serde_json::Value::String(s) = val {
        s
    }else{
        return Err(RunErr::from("Unexpected type for nontext data"));
    };

    Ok(data)
}


#[doc(hidden)]
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


#[doc(hidden)]
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


#[doc(hidden)]
fn print_nontext_match(filename: &std::ffi::OsString, m: &MatchedLine, cell: &Cell, icell: usize, cell_piece: &str, opts: &SearchOptions) {
    print_line_detail(filename, m, cell, icell, cell_piece, opts);
    print_colored("Non-text output data matches.");
    println!();
}


#[doc(hidden)]
fn trim_newline(s: &mut String) {
    // https://stackoverflow.com/a/55041833
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

#[doc(hidden)]
fn to_string_vec(a: &[&str]) -> Vec<String> {
    let mut tmp = Vec::new();
    for &el in a {
        tmp.push(String::from(el));
    }
    tmp
}

#[doc(hidden)]
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

#[doc(hidden)]
fn color_on(terminal: &mut std::boxed::Box<dyn term::Terminal<Output = std::io::Stdout> + std::marker::Send>) {
    terminal.fg(term::color::BRIGHT_RED).unwrap();
    terminal.attr(term::Attr::Bold).unwrap();
}

#[doc(hidden)]
fn color_off(terminal: &mut std::boxed::Box<dyn term::Terminal<Output = std::io::Stdout> + std::marker::Send>) {
    terminal.reset().unwrap();
}


#[doc(hidden)]
fn get_notebooks_in_dir(dirpath: &Path, file_list: &mut Vec<std::ffi::OsString>, recurse: bool) -> Result<(), RunErr> {
    let mut visited_dirs = HashSet::new();
    return get_notebooks_in_dir_internal(dirpath, file_list, recurse, &mut visited_dirs);
}

#[doc(hidden)]
fn get_notebooks_in_dir_internal(dirpath: &Path, file_list: &mut Vec<std::ffi::OsString>, recurse: bool, visited_dirs: &mut HashSet<std::ffi::OsString>) -> Result<(), RunErr> {
    // This *should* prevent infinite loops by not visiting a path more than once. 
    // I would have preferred using inodes, but those don't seem to be available -
    // maybe it's a unix-only thing, and since I'm using MUSL standard library,
    // it doesn't include those. I tested this by putting a symbolic link to a
    // directory inside itself and verified it did not search the notebooks in there
    // more than once.
    //
    // Inserting this into the set of visited paths at the beginning of the function
    // avoids an edge case where the directory visited >1 time is the top directory,
    // which doesn't get added to the set if we add it in the loop over directory 
    // entries
    let my_canon_path = std::ffi::OsString::from(dirpath.canonicalize()?);
    visited_dirs.insert(my_canon_path);
    for entry in dirpath.read_dir()? {
        if let Ok(entry) = entry {
            let entry_path = entry.path();
            if entry_path.is_dir() && recurse {
                let canon_path = std::ffi::OsString::from(entry_path.canonicalize()?);
                if !visited_dirs.contains(&canon_path){
                    get_notebooks_in_dir_internal(&entry_path, file_list, recurse, visited_dirs)?;
                }
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


#[doc(hidden)]
fn parse_clargs() -> Result<(Vec<std::ffi::OsString>, SearchOptions), RunErr> {
    let yml = clap::load_yaml!("clargs.yml");
    let clargs = clap::App::from_yaml(yml).version(clap::crate_version!()).get_matches();
    
    let opts = match SearchOptions::from_arg_matches(&clargs){
        Ok(o) => o,
        Err(e) => {
            let msg = format!("The search pattern was not valid: {}", e);
            return Err(RunErr{msg})
        }
    };

    let paths_raw = clargs.values_of_os("paths").unwrap();
    let mut paths: Vec<std::ffi::OsString> = Vec::new();
    for p in paths_raw {
        let curr_path = Path::new(p);
        if curr_path.is_file() {
            paths.push(std::ffi::OsString::from(p));
        }else if curr_path.is_dir() {
            get_notebooks_in_dir(curr_path, &mut paths, opts.recursive)?;
        } 
    }

    if paths.len() == 0 {
        return Err(RunErr{msg: "No notebook files listed or found in the given directories.".to_string()})
    }

    return Ok((paths, opts));
}

#[doc(hidden)]
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
