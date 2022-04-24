use alloc::{string::String, vec::Vec};

use crate::{print, println, task::keyboard};

use super::line_edit;

pub async fn run() {
    let stream = keyboard::KeyStream::new();
    let mut editor = line_edit::Editor::new(stream);

    let mut history = Vec::new();

    loop {
        let line = editor.prompt("> ").await;

        let (cmd, args) = match line.split_once(' ') {
            Some((cmd, args)) => (cmd, Some(args)),
            None => (line.as_str(), None),
        };

        if !cmd.is_empty() {
            run_command(cmd, args, &history).await;
        }

        history.push(line.clone());
    }
}

async fn run_command(cmd: &str, _args: Option<&str>, history: &[String]) {
    match cmd {
        "help" => {
            display_help();
        }
        "clear" => {
            print!("\x1b[2J");
        }
        "history" => {
            println!("{}", history.join("\n"));
        }
        _ => println!("unknown command: {}", cmd),
    }
}

fn display_help() {
    println!("Commands:");
    println!("  help");
    println!("  clear");
    println!("  history");
}
