use chrono::Local;

// bogus executable that doesn't drop privileges,
// doesn't create maildirs, and writes the delivered message
// to stdout.

use rattomail::{CreateMaildirsOption, MainContext, MessageDestination, PrivilegeOption};

fn main() {
    // bizarrely, bsd-mailx sets argv[0] to "send-mail", for no good reason.
    let valid_program_names = ["bogus_rattomail", "rattomail", "attomail", "sendmail", "send-mail"];
    let cli_args: Vec<String> = std::env::args().collect();
    let config_path = env!("ATTOMAIL_CONFIG_PATH");
    let now: chrono::DateTime<Local> = Local::now();

    let ctx = MainContext {
        args: cli_args,
        config_path: config_path.to_string(),
        should_drop_privs: PrivilegeOption::NoDropPrivileges,
        should_create_maildirs: CreateMaildirsOption::NoCreateMaildirs,
        message_destination: MessageDestination::OutputStream,
        received_time: now,
    };

    let stdin = std::io::stdin();
    let mut handle = stdin.lock();

    let mut stdout = std::io::stdout();

    rattomail::main(&valid_program_names, &ctx, &mut handle, Some(&mut stdout));
}
