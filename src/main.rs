extern crate getopts;
extern crate biowiki;

use std::env;
use std::path::PathBuf;
use getopts::Options;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("h", "host", "listen on host (default: localhost)", "HOST");
    opts.optopt("p", "port", "listen on port (default: 3000)", "PORT");
    opts.reqopt("d", "dir", "directory for wiki files", "PATH");
    opts.optflag("", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            println!("{}", f);
            print_usage(&program, opts);
            return;
        }
    };
    if matches.opt_present("help") {
        print_usage(&program, opts);
        return;
    }

    let host =
        if matches.opt_present("h") {
            matches.opt_str("h").unwrap()
        } else {
            "127.0.0.1".to_string()
        };

    let port =
        if matches.opt_present("p") {
            matches.opt_str("p").unwrap()
        } else {
            "3000".to_string()
        };

    let dir = matches.opt_str("d").unwrap();
    let path = PathBuf::from(dir);
    if !path.is_dir() {
        println!("{} is not a directory", path.to_str().unwrap());
        return;
    }

    biowiki::run(host, port, path);
}
