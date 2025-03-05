use libc::{c_int, signal, SIGINT};
use std::io::{self, Write};
use std::{env, fs};

fn get_prompt_string(prompt: &mut String) {
    prompt.clear();

    let blue: &str = "\x1b[1;34m";
    let green: &str = "\x1b[1;32m";
    let reset: &str = "\x1b[0m";

    prompt.push_str(green);

    match env::var("USER") {
        Ok(user) => {
            prompt.push_str(&user);
            prompt.push('@');
        }
        Err(_) => {
            return;
        }
    }

    match fs::read_to_string("/proc/sys/kernel/hostname") {
        Ok(hostname) => {
            prompt.push_str(hostname.trim());
            prompt.push(':');
        }
        Err(_) => {
            prompt.clear();
            return;
        }
    }
    prompt.push_str(blue);

    match env::current_dir() {
        Ok(path) => {
            prompt.push_str(&path.to_string_lossy());
            prompt.push(' ');
        }
        Err(_) => {
            prompt.clear();
            return;
        }
    }
    prompt.push_str(reset);
}

extern "C" fn handle_sigint(_sig: c_int) {
    let mut prompt: String = String::new();
    get_prompt_string(&mut prompt);
    print!("\n{prompt}$ ");
    match io::stdout().flush() {
        _ => {}
    }
}

fn tokenize_comm(line: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut chars = line.char_indices().peekable();

    let mut ch: char;
    let mut ind: usize = 0;
    let mut start: usize = 0;
    let mut is_quote: bool = false;

    'outer: loop {
        match chars.peek() {
            None => {
                if start < ind {
                    tokens.push(line[start..=ind].to_string());
                }
                break 'outer;
            }
            Some((i, c)) => {
                ind = *i;
                ch = *c;
            }
        }

        match ch {
            ' ' | '\t' => {
                if is_quote {
                    chars.next();
                    continue;
                } else if start == ind {
                    start = ind + 1;
                    chars.next();
                    continue;
                }
                tokens.push(line[start..ind].to_string());
                start = ind + 1;
            }

            '"' | '\'' => {
                is_quote = !is_quote;
                chars.next();
            }
            _ => {}
        }
        chars.next();
    }

    tokens
}

fn split_subcommands(line: &str) -> Vec<String> {
    let mut subcoms: Vec<String> = Vec::new();
    let mut chars = line.char_indices().peekable();

    let mut ch: char;
    let mut ind: usize = 0;

    let mut start: usize = 0;
    let mut is_quote: bool = false;

    'outer: loop {
        match chars.peek() {
            None => {
                if start < ind {
                    subcoms.push(line[start..=ind].to_string());
                    tokenize_comm(&line[start..=ind]);
                }
                break 'outer;
            }
            Some((i, c)) => {
                ind = *i;
                ch = *c;
            }
        }

        match ch {
            '|' => {
                if is_quote {
                    chars.next();
                    continue;
                }
                subcoms.push(line[start..ind].to_string());
                tokenize_comm(&line[start..ind]);
                start = ind + 1;
            }
            '"' | '\'' => {
                is_quote = !is_quote;
            }
            _ => {}
        }
        chars.next();
    }

    subcoms
}

fn read_input(prompt: &str, buf: &mut String) -> usize {
    print!("{}$ ", prompt);

    buf.clear();

    match io::stdout().flush() {
        Err(e) => {
            println!("{e}");
            return 0;
        }
        _ => {}
    }

    match io::stdin().read_line(buf) {
        Err(e) => {
            println!("{e}");
            0
        }
        Ok(len) => len,
    }
}

fn main() {
    unsafe {
        signal(SIGINT, handle_sigint as usize);
    }

    let mut buf: String = String::new();
    let mut prompt: String = String::new();

    get_prompt_string(&mut prompt);

    loop {
        if read_input(&prompt, &mut buf) == 0 {
            println!("");
            break;
        }

        let line: &str = buf.trim();
        if line.len() == 0 {
            continue;
        }

        let _sub_coms: Vec<String> = split_subcommands(line);
    }
}
