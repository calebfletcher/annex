use alloc::{borrow::ToOwned, string::String, vec::Vec};

use crate::{cmos, emulators, hpet, print, println, task::keyboard};

use super::line_edit;

pub async fn run() {
    let mut editor = line_edit::Editor::new(keyboard::KeyStream::new());

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
        "time" => {
            println!(
                "Local Time: {}",
                cmos::RTC
                    .try_get()
                    .map(|rtc| rtc
                        .time()
                        .with_timezone(&chrono_tz::Australia::Melbourne)
                        .to_rfc3339())
                    .unwrap_or_else(|_| "unavailable".to_owned())
            );
        }
        "mmap" => {
            // for region in kernel_memory_map.iter() {
            //     let size_bytes = region.end - region.start;
            //     let size = memory::format_bytes(size_bytes);
            //     serial_println!(
            //         "{:018p}-{:018p} ({}): {:?}",
            //         region.start as *const u8,
            //         region.end as *const u8,
            //         size,
            //         region.kind
            //     );
            // }
            println!("unimplemented");
        }
        "q" | "quit" => {
            emulators::exit_qemu(emulators::QemuExitCode::Success);
        }
        "ts" => {
            println!("ts {} {}", hpet::get(), hpet::get_seconds());
        }
        _ => println!("unknown command: {}", cmd),
    }
}

fn display_help() {
    println!("Commands:");
    println!("  help");
    println!("  clear");
    println!("  history");
    println!("  time");
}
