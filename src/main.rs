use libc::{c_int, signal, SIGINT};
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
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

fn tokenize_comm(comm: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut chars = comm.char_indices().peekable();

    let mut ch: char;
    let mut ind: usize = 0;
    let mut start: usize = 0;
    let mut is_quote: bool = false;

    'outer: loop {
        match chars.peek() {
            None => {
                if start < ind {
                    tokens.push(comm[start..=ind].to_string());
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
                    continue 'outer;
                } else if start == ind {
                    start = ind + 1;
                    chars.next();
                    continue 'outer;
                }
                tokens.push(comm[start..ind].to_string());
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
    let mut subcomms: Vec<String> = Vec::new();
    let mut chars = line.char_indices().peekable();

    let mut ch: char;
    let mut ind: usize = 0;

    let mut start: usize = 0;
    let mut is_quote: bool = false;

    'outer: loop {
        match chars.peek() {
            None => {
                if start < ind {
                    subcomms.push(line[start..=ind].to_string());
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
                    continue 'outer;
                }
                subcomms.push(line[start..ind].to_string());
                start = ind + 1;
            }
            '"' | '\'' => {
                is_quote = !is_quote;
            }
            _ => {}
        }
        chars.next();
    }

    subcomms
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

    'repl: loop {
        if read_input(&prompt, &mut buf) == 0 {
            println!("");
            break 'repl;
        }

        let line: &str = buf.trim();
        if line.len() == 0 {
            continue 'repl;
        }

        let sub_comms: Vec<String> = split_subcommands(line);
        let mut prev_out = None;

        for (i, comm) in sub_comms.iter().enumerate() {
            let tokens = tokenize_comm(&comm);

            if let Some((comm, args)) = tokens.split_first() {
                let stdin: Stdio;
                let stdout: Stdio;

                match prev_out.take() {
                    Some(out) => {
                        stdin = Stdio::from(out);
                    }
                    None => {
                        stdin = Stdio::inherit();
                    }
                }

                if i == sub_comms.len() - 1 {
                    stdout = Stdio::inherit();
                } else {
                    stdout = Stdio::piped();
                }

                let child = Command::new(comm)
                    .args(args)
                    .stdin(stdin)
                    .stdout(stdout)
                    .spawn();

                match child {
                    Ok(mut proc) => {
                        prev_out = proc.stdout.take();
                        let _ = proc.wait();
                    }
                    Err(e) => {
                        eprintln!("{e}");
                    }
                }
            }
        }
    }
}
