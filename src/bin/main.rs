extern crate sql_lexer;

use std::env;
use std::io::prelude::*;
use std::fs::File;

use sql_lexer::*;

fn main() {
    match env::args().skip(1).next() {
        Some(arg) => {
            let mut file = File::open(arg.as_str()).unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            println!("{}", sanitize_string(contents));
        },
        None => println!("Please supply a path to a file containing SQL")
    }
}
