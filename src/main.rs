use std::{env, fs, process::exit};

mod json;

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("expected file path at arg 1");
            exit(1);
        }
    };

    let json_str = fs::read_to_string(path).unwrap();
    match json::parse(&json_str) {
        Err(why) => {
            eprintln!("{}", why);
            exit(1);
        }
        Ok(v) => println!("{:?}", v),
    }
}
