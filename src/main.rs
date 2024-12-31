use chrono::Local;

use rattomail::{CreateMaildirsOption, MainContext, MessageDestination, PrivilegeOption};

fn main() {
    // bizarrely, bsd-mailx sets argv[0] to "send-mail", for no good reason.
    let valid_program_names = ["rattomail", "attomail", "sendmail", "send-mail"];
    let cli_args: Vec<String> = std::env::args().collect();
    let config_path = env!("ATTOMAIL_CONFIG_PATH");
    let now: chrono::DateTime<Local> = Local::now();

    let ctx = MainContext {
        args: cli_args,
        config_path: config_path.to_string(),
        should_drop_privs: PrivilegeOption::DropPrivileges,
        should_create_maildirs: CreateMaildirsOption::CreateMaildirs,
        message_destination: MessageDestination::Maildir,
        received_time: now,
    };

    let stdin = std::io::stdin();
    let mut handle = stdin.lock();

    let output_opt: Option<&mut std::io::Stdout> = None;

    rattomail::main(&valid_program_names, &ctx, &mut handle, output_opt);
}
