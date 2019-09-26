mod shells;
mod visit;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use env_logger;
use libc;
use log::{debug, error};

use shells::SUPPORTED_SHELLS;

fn main() {
    match run() {
        Ok(()) => {
            std::process::exit(0);
        }
        Err(code) => {
            std::process::exit(code);
        }
    }
}

fn run() -> Result<(), i32> {
    let flags = App::new("driven")
        .version("0.1")
        .author("Euan Kemp")
        .about("Set environment variables based on the directory you're in.")
        .arg(Arg::with_name("debug").help("Enable debug output"))
        .subcommand(
            SubCommand::with_name("init")
                .about("Prints initialization logic for the given shell to eval")
                .usage(format!("driven init [ {} ]", SUPPORTED_SHELLS.join(" | ")).as_str())
                .arg(Arg::with_name("shell").help(&format!(
                    "the shell to print initialization code for: one of {}",
                    SUPPORTED_SHELLS.join(", ")
                ))),
        )
        .subcommand(
            SubCommand::with_name("visit")
                // used by the shell hooks internally, it shouldn't be called directly
                .setting(AppSettings::Hidden)
                .setting(AppSettings::DisableHelpSubcommand)
                .about("visit a directory, export variables")
                .arg(Arg::with_name("shell").long("shell").takes_value(true).help(&format!(
                    "the shell to print initialization code for: one of {}",
                    SUPPORTED_SHELLS.join(", ")
                )))
                .arg(Arg::with_name("dir_target")),
        )
        .get_matches();

    if flags.is_present("debug") {
        env_logger::Builder::new()
            .filter(None, log::LevelFilter::Debug)
            .try_init()
            .unwrap();

        // Capture ctrl-c so calling script
        // can print debug output
        if let Err(()) = intercept_ctrl_c() {
            return Err(2);
        }
    }

    match flags.subcommand() {
        ("init", Some(init)) => handle_init(init),
        ("visit", Some(visit)) => handle_visit(visit),
        _ => {
            println!("unrecognized subcommand");
            Err(1)
        }
    }
}

fn handle_init(cmd: &ArgMatches) -> Result<(), i32> {
    match cmd.value_of("shell") {
        Some(s) => match shells::from_name(s) {
            Some(s) => {
                println!("{}", s.driven_init());
                return Ok(());
            }
            None => {
                println!("{}\n\nUnsupported shell: {}", cmd.usage(), s);
                return Err(1);
            }
        },
        None => {
            println!("{}\n\ninit requires an argument", cmd.usage());
            return Err(1);
        }
    }
}

fn handle_visit(cmd: &ArgMatches) -> Result<(), i32> {
    let shell = shells::from_name(cmd.value_of("shell").unwrap()).unwrap();
    match cmd.value_of("dir_target") {
        Some(dir) => {
            match visit::visit(shell, &mut std::io::stdout(), dir) {
                Err(e) => {
                    error!("{}", e);
                    Err(1)
                }
                Ok(_) => {
                    Ok(())
                }
            }
        }
        None => {
            error!("{}\n\nvisit requires an argument", cmd.usage());
            Err(1)
        }
    }
}

fn intercept_ctrl_c() -> Result<(), ()> {
    // When Pazi is run from a script or shell function,
    // pressing ctrl-c will send SIGINT to the process group
    // containing both Pazi *and* the shell function.
    //
    // However, sometimes we just want to SIGINT Pazi but
    // allow the caller to keep running (e.g., to print output).
    // To accomplish this, we need to put Pazi in its own
    // process group and make that the foreground process group.
    // That way, when ctrl-c sends a SIGINT, the only process
    // to receive it is Pazi.
    //
    unsafe fn get_errno() -> *mut libc::c_int {
        #[cfg(target_os = "linux")]
        return libc::__errno_location();
        #[cfg(target_os = "macos")]
        return libc::__error();
    }

    unsafe {
        // If STDIN isn't a tty, we can't reasonably make ourselves
        // the foreground process group, so just give up
        // (happens during zsh autocompletion)
        let isatty_res = libc::isatty(libc::STDIN_FILENO);
        if isatty_res == 0 {
            return Ok(());
        }

        // Create a new process group with this process in it.
        let setpgid_res = libc::setpgid(0, 0);
        let errno = *get_errno();
        if setpgid_res != 0 {
            debug!("Got {} from setpgid with errno {}", setpgid_res, errno);
            return Err(());
        }

        // Get the ID of the process group we just made.
        let pgrp = libc::getpgrp();

        // Make this process group the foreground process.
        // SIGTTOU is sent if this process group isn't already foreground,
        // so we ignore it during the change.

        // New SIGTTOU handler that ignores the signal
        let ignore_action = libc::sigaction {
            sa_sigaction: libc::SIG_IGN,
            sa_mask: std::mem::zeroed(),
            sa_flags: 0,
            #[cfg(target_os = "linux")]
            sa_restorer: None,
        };
        // Place to save old SIGTTOU handler
        let mut old_action = std::mem::zeroed();

        // Ignore SIGTTOU and save previous action
        let sigaction_res = libc::sigaction(libc::SIGTTOU, &ignore_action, &mut old_action);
        let errno = *get_errno();
        if sigaction_res != 0 {
            debug!("Got {} from sigaction with errno {}", sigaction_res, errno);
            return Err(());
        }

        // Make our process group the foreground process group
        // (giving us access to stdin, etc)
        let tcsetpgrp_res = libc::tcsetpgrp(libc::STDIN_FILENO, pgrp);
        let errno = *get_errno();

        // Put the old SIGTTOU signal handler back
        // We try to do this even if tcsetpgrp failed!
        let sigaction_res = libc::sigaction(libc::SIGTTOU, &old_action, std::ptr::null_mut());
        let sigaction_errno = *get_errno();

        // Handle tcsetpgrp and sigaction errors
        if tcsetpgrp_res != 0 || sigaction_res != 0 {
            debug!("Got pgrp {}", pgrp);
            debug!("Got {} from tcsetpgrp with errno {}", tcsetpgrp_res, errno);
            debug!(
                "Got {} from sigaction with errno {}",
                sigaction_res, sigaction_errno
            );
            return Err(());
        }
    }

    Ok(())
}
