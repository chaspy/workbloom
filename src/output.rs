use std::sync::atomic::{AtomicBool, Ordering};
use std::process::{Command, Stdio};

static MACHINE_OUTPUT: AtomicBool = AtomicBool::new(false);

pub fn set_machine_output(enabled: bool) {
    MACHINE_OUTPUT.store(enabled, Ordering::Relaxed);
}

pub fn is_machine_output() -> bool {
    MACHINE_OUTPUT.load(Ordering::Relaxed)
}

pub fn configure_command_for_machine_output(command: &mut Command) -> &mut Command {
    if is_machine_output() {
        command.stdout(Stdio::null());
        command.stderr(Stdio::inherit());
    }
    command
}

#[macro_export]
macro_rules! outln {
    () => {
        if $crate::output::is_machine_output() {
            eprintln!();
        } else {
            println!();
        }
    };
    ($($arg:tt)*) => {
        if $crate::output::is_machine_output() {
            eprintln!($($arg)*);
        } else {
            println!($($arg)*);
        }
    };
}
