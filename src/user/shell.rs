use core::fmt::Write;

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};

use crate::{acpi, cmos, emulators, gui, hpet, screen::Terminal, task::keyboard, threading};

use super::line_edit;

pub async fn run() {
    let mut shell = Shell::new();
    shell.run().await;
}

struct Shell {
    terminal: Terminal,
    editor: line_edit::Editor<keyboard::KeyStream>,
    history: Vec<String>,
}

impl Shell {
    fn new() -> Self {
        let screen = gui::SCREEN.try_get().unwrap();
        let window = screen
            .lock()
            .new_window(gui::Coordinates::new(100, 300, 400, 200));
        let terminal = Terminal::new(window);

        let editor = line_edit::Editor::new(keyboard::KeyStream::new());
        let history = Vec::new();

        Self {
            terminal,
            editor,
            history,
        }
    }

    pub async fn run(&mut self) {
        loop {
            let line = self.editor.prompt("> ", &mut self.terminal).await;

            let (cmd, args) = match line.split_once(' ') {
                Some((cmd, args)) => (cmd, Some(args)),
                None => (line.as_str(), None),
            };

            if !cmd.is_empty() {
                self.run_command(cmd, args).await;
            }

            self.history.push(line.clone());
        }
    }

    async fn run_command(&mut self, cmd: &str, _args: Option<&str>) {
        match cmd {
            "help" => {
                self.display_help();
            }
            "clear" => {
                self.terminal.write_str("\x1b[2J").unwrap();
            }
            "history" => {
                self.terminal.write_str(&self.history.join("\n")).unwrap();
            }
            "time" => {
                self.terminal
                    .write_str(&format!(
                        "Local Time: {}\n",
                        cmos::RTC
                            .try_get()
                            .map(|rtc| {
                                rtc.time()
                                    .with_timezone(&chrono_tz::Australia::Melbourne)
                                    .to_rfc3339()
                            })
                            .unwrap_or_else(|_| "unavailable".to_owned()),
                    ))
                    .unwrap();
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
                self.terminal.write_str("unimplemented\n").unwrap();
            }
            "q" | "quit" => {
                emulators::exit_qemu(emulators::QemuExitCode::Success);
            }
            "ts" => {
                self.terminal
                    .write_str(&format!("time since boot: {:.3} s\n", hpet::seconds()))
                    .unwrap();
            }
            "shutdown" => {
                self.terminal
                    .write_str("shutdown currently not working\n")
                    .unwrap();
                acpi::ACPI.try_get().unwrap().try_lock().unwrap().shutdown();
            }
            "ps" | "processes" => {
                let threads = threading::scheduler::with_scheduler(|s| s.to_view());

                for thread in threads.iter() {
                    let time = thread.time() as f64 / 1e9;
                    self.terminal
                        .write_str(&format!(
                            "{:?} | {} | {:?} | {:.3}s\n",
                            thread.id(),
                            thread.name(),
                            thread.state(),
                            time
                        ))
                        .unwrap();
                }
            }
            _ => {
                self.terminal
                    .write_str(&format!("unknown command: {}\n", cmd))
                    .unwrap();
            }
        };
    }

    fn display_help(&mut self) {
        self.terminal.write_str("Commands:\n").unwrap();
        self.terminal
            .write_str("  help - display this list\n")
            .unwrap();
        self.terminal.write_str("  clear - clear screen\n").unwrap();
        self.terminal
            .write_str("  history - shell history\n")
            .unwrap();
        self.terminal.write_str("  time - get wall time\n").unwrap();
        self.terminal
            .write_str("  ts - get time since boot\n")
            .unwrap();
        self.terminal.write_str("  quit - exit vm\n").unwrap();
        self.terminal
            .write_str("  shutdown - shutdown hardware\n")
            .unwrap();
        self.terminal
            .write_str("  processes - show process info\n")
            .unwrap();
    }
}
