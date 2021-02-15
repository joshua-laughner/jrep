# jrep
**jrep** is grep for Jupyter notebooks. It is a command line program that can search across 
multiple notebooks for specific text, but limit itself to certain types of cells, source text,
output data, or any combination. This is built to avoid `grep` matching strings of characters
in output data (like images).

## Installing
If you are using a 64-bit Linux, you can try downloading and unzipping the latest release and 
placing the `jrep` file somewhere on your [PATH](https://linuxhint.com/path_in_bash/). For the
moment, any other platforms will need to download the source code and compile it locally. This
requires [Rust](https://www.rust-lang.org/) be installed, with that, running `make release` in
the repo will compile the program. The compiled program will be `target/release/jrep` in the 
repo folder.

## Quick start

Once the program is in a directory on your `PATH` you can call it very similarly to `grep`. The
minimum is to give it a pattern to search for and one or more files to search. The pattern is treated
as a regular expression. This is implemented with the Rust `regex` crate, so the syntax it uses is
described [here](https://docs.rs/regex/1.4.3/regex/#syntax).

* Search the notebook `example.ipynb` for any mention of "CO2":

```
jrep CO2 example.ipynb
```

* Search all notebooks in the current directory for "co2" ignoring case:

```
jrep -i co2 *.ipynb
```

* Search notebooks for dates in YYYY-MM-DD or YYYYMMDD format. Note that the search pattern is
  in quotes - this is because terminal shells usually treat strings like `\d` as a special character
  instead of a literal backslash+d. 
  
 ```
 jrep '\d{4}-?\d{2}-?\d{2}' *.ipynb
 ```
 
 For a list of available options, use `jrep -h` or `jrep --help`. Where practical, `jrep` mimics `grep`
 command line options.
 
 ## Limitations
 
 This is still an early version, so it probably will not work in some cases - some notebooks may be missing
 expected elements or have a different structure than expected. Not all `grep` options have been implemented;
 some (like `--count` and the context options) are planned, others will not be added because they do not make
 sense for notebooks.
