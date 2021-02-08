use std::fs;

use serde::{Deserialize, Serialize};

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
    include_source: bool,
    include_output: bool
}

#[derive(Serialize, Deserialize)]
struct Notebook {
    cells: Vec<Cell>
}

#[derive(Serialize, Deserialize)]
struct Cell {
    cell_type: String,
    source: Vec<String>
}


fn load_notebook(path: &str) -> Result<Notebook, RunErr>{
    let data = fs::read_to_string(path)?;
    let notebook: Notebook = serde_json::from_str(&data)?;

    Ok(notebook)
}


fn main() {
    let nb = load_notebook("example-notebooks/demo.ipynb").unwrap();
    println!("Notebook has {} cells", nb.cells.len());
    println!("Cell 0 source = {:#?}", nb.cells[0].source);
}
