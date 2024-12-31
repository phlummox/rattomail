use std::env;
use std::fs::File;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use clap::{command, Arg, ArgAction, Command};
use ini::Ini;
use maildir::Maildir;
use nix::unistd::{Uid, User};
use simplelog::{LevelFilter, WriteLogger};

/// Contents of a config file.
///
/// - `mailDir` is a path to a Maildir/new directory.
/// - `userName` is the name of the user we'll assume the privileges of while delivering mail
#[derive(Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct Config {
    pub mailDir: String,
    pub userName: String,
}

/// Whether to drop privileges (i.e., change to the user specified in the config file).
/// In production, we should always drop privileges; in testing, we might not.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum PrivilegeOption {
    NoDropPrivileges,
    DropPrivileges,
}

/// Whether to create Maildir directories, if they don't exist.
/// In production, we should always create them; in testing, we might not.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CreateMaildirsOption {
    NoCreateMaildirs,
    CreateMaildirs,
}

/// Where to write the message to.
/// In production, this should be `Maildir`; in testing, we might
/// instead write to some `OutputStream`.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum MessageDestination {
    Maildir,
    OutputStream,
}

/// Main context for the program. Represents values injected into main() for easy testing.
///
/// Fields:
///
/// - `args`: command-line arguments
/// - `config_path`: path to a config file
/// - `should_drop_privs`: whether to drop privileges (i.e., change to the user specified
///   in the config file)
/// - `should_create_maildirs`: whether to create any necessary Maildir directories (if
///   they don't exist)
/// - `message_destination`: where to deliver mail to (maildirs or an output stream)
/// - `received_time`: time the program was invoked. Used as the "Received" time in headers,
///    and for the `Date:` header if we need to insert one.
#[derive(Debug)]
pub struct MainContext {
    pub args: Vec<String>,
    pub config_path: String,
    pub should_drop_privs: PrivilegeOption,
    pub should_create_maildirs: CreateMaildirsOption,
    pub message_destination: MessageDestination,
    pub received_time: chrono::DateTime<Local>,
}

/// Normalize the program name to one of the names we expect to be invoked as:
/// e.g. `rattomail`, `attomail`, or `sendmail`. If the name is not one of these, exit with an
/// error message.
fn normalize_prog_name(valid_names: &[&str], prog_name: &String) -> String {
    // last component of program's path
    let last_component = Path::new(&prog_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| {
            eprintln!("Error: Invalid program name '{}'.", prog_name);
            std::process::exit(1);
        });

    // Check if the last component matches any of the valid names
    if valid_names.contains(&last_component) {
        return last_component.to_string();
    }

    // invalid program name
    eprintln!(
        "Error: Invalid program name '{}'. Only {:?} are allowed.",
        prog_name, valid_names
    );
    std::process::exit(1);
}

/// Build a CLI parser for the program.
/// Most of the arguments exist only for compatibility with sendmail, and are ignored.
/// The only arguments we actually use are `-f`, `-bm`, and '-X', and (if present) a
/// positional argument, the recipient address.
///
/// The `-f` argument is used to specify the sender (from) envelope address. The address
/// must not contain non-ASCII-graphical characters (see <https://doc.rust-lang.org/std/primitive.char.html#method.is_ascii_graphic>
/// or libc isgraphic).
/// If not specified, the program
/// should use the current user's username (which is checked once privileges are dropped,
/// and we've changed to the user specified in the config file).
///
/// The `-bm` argument is used to specify the mode of operation. If `-bm` or `-b m` is supplied, the program
/// will read the message from stdin (which is the default mode of operation anyway).
/// Given any other argument to `-b`, the program should print an error message and exit.
///
/// The `-X` argument is used to specify a logfile. The only permissible arguments for
/// rattomail are `/dev/stderr` and '-' (which has the same meaning as `/dev/stderr`).
/// Given any other argument, the program should print an error message and exit.
///
/// Polite user-mail agents will normally provide the recipient address, but because
/// some don't (e.g. bsd-mailx), we don't mandate it.
pub fn build_cli() -> Command {
    command!()
    .disable_version_flag(true)
    .arg(
        Arg::new("version")
            .long("version")
            .action(ArgAction::Version)
            .help("Print version")
    )

    // actual args we use - `-f sender`, `-bm`, and `-X logfile`
    .arg(Arg::new("sender_env").short('f').value_name("ADDRESS")
        .help("Sender (from) envelope address. If not specified, the current user is used. Must not contain non-ASCII, whitespace or non-printable characters."))
    .arg(Arg::new("b").short('b').value_name("MODE")
        .help("-bm: Read input from stdin (default), everything else - error"))
    .arg(Arg::new("logfile").short('X').value_name("LOGFILE")
        .help("Log debugging messages to a file. The only valid values are /dev/stderr and '-', which has the same meaning. (Originally: 'Log mailer traffic')"))

    // ignored args that take no argument - i, n, t
    .arg(Arg::new("i").short('i')
        .action(ArgAction::SetTrue)
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'Ignore dots alone on lines by themselves in incoming messages.')"))
    .arg(Arg::new("n").short('n')
        .action(ArgAction::SetTrue)
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'Don't do aliasing.')"))
    .arg(Arg::new("t").short('t')
        .action(ArgAction::SetTrue)
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'Read message to work out the recipients.')"))

    // ignored args that do take an argument - o, p, q, r, v, B, C, F, N, O, R, U, V, X
    .arg(Arg::new("o").short('o')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'set an option')"))
    .arg(Arg::new("p").short('p')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'specify PROTOCOL')"))
    .arg(Arg::new("q").short('q')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'used to specify a queue interval')"))
    .arg(Arg::new("r").short('r')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'obsolete equivalent to -f, to specify sender envelope')"))
    .arg(Arg::new("v").short('v')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'obsolete equivalent to -f, to specify sender envelope')"))
    .arg(Arg::new("B").short('B')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'set body type to 7BIT or 8BITMIME')"))
    .arg(Arg::new("C").short('C')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'use an alternate configuration file')"))
    .arg(Arg::new("F").short('F')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'set full name of sender')"))
    .arg(Arg::new("N").short('N')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'specify delivery status notification conditions')"))
    .arg(Arg::new("O").short('O')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'set an option')"))
    .arg(Arg::new("R").short('R')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'set amount of the message to be returned if the message bounces')"))
    .arg(Arg::new("U").short('U')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'ignored - initial user submission')"))
    .arg(Arg::new("V").short('V')
        .help("Ignored, used only for compatibility with sendmail. (Originally: 'set envelope ID for notification')"))

    // positional arguments - to address
    .arg(Arg::new("to_address")
         .value_name("RECIPIENT")
         .help("Recipient address")
         .required(false))
}

/// Read a "key = value" style config file, and return the values as a Config struct.
///
/// The file must contain a section with the following keys:
///   - mailDir: path to a subdir of a Maildir directory, where new mail will be stored
///   - userName: name of the user we expect the Maildir to be owned by. (When deliviering mail,
///     the program will attempt to drop privileges and run as this user.)
///
pub fn read_config_ini<P>(file_path: P) -> Result<Config>
where
    P: AsRef<Path>,
{
    let file_path_ref = file_path.as_ref();
    let conf = Ini::load_from_file(file_path_ref).map_err(|e| {
        anyhow::anyhow!(
            "Error reading config file {}: {}",
            file_path_ref.display().to_string(),
            e
        )
    })?;

    let section = conf.section(None::<String>).ok_or_else(|| {
        anyhow!(
            "Error reading config file {}: sections seem malformed",
            file_path_ref.display()
        )
    })?;
    let mail_dir = section.get("mailDir").ok_or_else(|| {
        anyhow!(
            "Error reading config file {}: variable mailDir not found",
            file_path_ref.display()
        )
    })?;

    let user_name = section.get("userName").ok_or_else(|| {
        anyhow!(
            "Error reading config file {}: variable userName not found",
            file_path_ref.display()
        )
    })?;

    let config = Config {
        mailDir: mail_dir.to_string(),
        userName: user_name.to_string(),
    };

    Ok(config)
}

/// Return the username of the current user, or exit with an error message.
/// Exits the program, with an error message, on failure.
pub fn get_current_user() -> String {
    // Getting the current user's username is basically infallible, unless
    // something has gone terribly wrong; testing failure scenarios is
    // tricky; and shifting error-handling logic into `main` has little benefit.
    // So we just exit with an error message if it happens.

    let uid: Uid = Uid::current();
    let user: User = User::from_uid(uid).map_or_else(
        |err| {
            let desc = err.desc();
            eprintln!(
                "Couldn't get username for uid {}: errno was {} ({})",
                uid, err, desc
            );
            std::process::exit(1);
        },
        |opt| {
            opt.unwrap_or_else(|| {
                eprintln!("Couldn't get username for uid {}: no such user", uid);
                std::process::exit(1);
            })
        },
    );
    user.name
}

/// set up logging for a given logfile path. The only permissible paths, however, are
/// `/dev/stderr` and `-` (which is equivalent to `/dev/stderr`). Any other path will
/// cause the program to exit with an error message.
fn init_logfile(logfile_path: String) {
    let valid_logfiles = ["-", "/dev/stderr"];

    if !valid_logfiles.contains(&logfile_path.as_str()) {
        eprintln!(
            "Error: Invalid logfile path '{}'. Only {:?} are allowed.",
            logfile_path, valid_logfiles
        );
        std::process::exit(1);
    }

    let logfile_path = if logfile_path == "-" {
        "/dev/stdout".to_string()
    } else {
        logfile_path
    };

    let logfile = File::create(logfile_path.clone()).unwrap_or_else(|e| {
        eprintln!("Error creating log file '{}': {}", logfile_path, e);
        std::process::exit(1);
    });
    let _ = WriteLogger::init(LevelFilter::Trace, simplelog::Config::default(), logfile);
}

/// Drop privileges to the specified user. If the specified user is root, exit with an error message.
/// If an error occurs while dropping privileges, exit with an error message.
fn drop_privileges(new_user: User) {
    // We attempt to follow the recipe laid out in Viega et al, Secure Programming Cookbook for C and C++
    // (O'Reilly, 2003), recipe 1.3, "Dropping Privileges in setuid Programs".
    // We drop all ancillary groups, then the group privileges, then the user privileges,
    // and finally check that we can't regain them.

    let old_uid = nix::unistd::geteuid();
    let old_gid = nix::unistd::getegid();

    let new_uid = new_user.uid;

    if new_uid.is_root() {
        eprintln!("Error: Cannot run as root. Please specify a different user in the config file.");
        std::process::exit(1);
    }

    let new_gid = new_user.gid;

    // drop ancillary groups from process
    nix::unistd::setgroups(&[new_gid]).unwrap_or_else(|e| {
        eprintln!("Error: Couldn't drop ancillary groups: {}", e);
        std::process::exit(1);
    });

    nix::unistd::setresgid(new_gid, new_gid, new_gid).unwrap_or_else(|e| {
        eprintln!("Error: Couldn't drop group privileges: {}", e);
        std::process::exit(1);
    });

    nix::unistd::setresuid(new_uid, new_uid, new_uid).unwrap_or_else(|e| {
        eprintln!("Error: Couldn't drop user privileges: {}", e);
        std::process::exit(1);
    });

    // check that privileges can't be regained

    if new_gid != old_gid {
        let res = nix::unistd::setresgid(old_gid, old_gid, old_gid);
        match res {
            Ok(_) => {
                eprintln!(
                    "Error: Failed to drop group privileges: setresgid of old gid {} succeeded unexpectedly",
                    old_gid
                );
                std::process::exit(1);
            }
            Err(_e) => {}
        }
    }

    if new_uid != old_uid {
        let res = nix::unistd::setresuid(old_uid, old_uid, old_uid);
        match res {
            Ok(_) => {
                eprintln!(
                    "Error: Failed to drop user privileges: setresuid of old uid {} succeeded unexpectedly",
                    old_uid
                );
                std::process::exit(1);
            }
            Err(_e) => {}
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct HeaderStatus {
    pub has_from: bool,
    pub has_date: bool,
}

/// Read headers from an input stream, and write them to an output stream, recording whether
/// we've seen the `From:` and `Date:` headers.
///
/// Should write all the header lines to the output stream, _except_ for the final newline
/// indicating the end of the headers. (Because the caller will want to write additional
/// headers after this function returns.)
///
/// So if `Foo: foo\nBar: bar\n\n` is read from the input, `Foo: foo\nBar: bar\n` should be
/// written to the output.
///
/// Returns a `HeaderStatus` struct indicating whether we've seen the `From:` and `Date:` headers.
/// If an error occurs while reading or writing, returns an error.
///
/// Example
///
/// ```
/// use std::io::Cursor;
/// use rattomail::{process_existing_headers,HeaderStatus};
///
/// let input = b"Foo: foo\nBar: bar\n\n";
/// let mut output = Vec::new();
/// let result = process_existing_headers(&mut Cursor::new(input), &mut output).unwrap();
///
/// assert_eq!(result, HeaderStatus { has_from: false, has_date: false });
/// assert_eq!(output, b"Foo: foo\nBar: bar\n");
/// ```
///
pub fn process_existing_headers<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
) -> Result<HeaderStatus> {
    let mut buffer = Vec::new();
    // record what headers we see
    let mut header_status = HeaderStatus {
        has_from: false,
        has_date: false,
        //reached_header_end: false,
    };

    loop {
        // read until newline or EOF
        let bytes_read = input
            .read_until(b'\n', &mut buffer)
            .map_err(|e| anyhow!("Error reading input: {}", e))?;

        // check for headers
        if buffer.starts_with(b"From: ") {
            header_status.has_from = true;
        } else if buffer.starts_with(b"Date: ") {
            header_status.has_date = true;
        } else if buffer == b"\n" || buffer == b"\r\n" {
            // end of headers
            break;
        }

        if bytes_read == 0 {
            break; // reached EOF
        }

        output
            .write_all(&buffer)
            .map_err(|e| anyhow!("Error writing output: {}", e))?;

        // clear for next read
        buffer.clear();
    }

    // ensure all buffered data is written
    output
        .flush()
        .map_err(|e| anyhow!("Error flushing output: {}", e))?;

    Ok(header_status)
}

/// Make a `Received:` header for a given `to_addr`, `from_addr`, and `time`.
pub fn make_received_header(
    to_addr: &str,
    from_addr: &str,
    time: &chrono::DateTime<Local>,
) -> String {
    let date_str = time.to_rfc2822();
    format!(
        "Received: for {} with local (rattomail) (envelope-from {}); {}\n",
        to_addr, from_addr, date_str
    )
}

/// Write a `Received:` header to the output stream, then existing headers
/// (read from input stream), plus `Date:` and `From:` headers if missing,
/// then a blank line terminator to indicate end of headers.
///
/// The current time is used to get a date-time for the `Received` header.
///
/// Arguments:
///
/// - `input`: input stream to read existing headers from
/// - `output`: output stream to write headers to
/// - `to_addr`: recipient address
/// - `from_addr`: sender address
pub fn write_headers<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    to_addr: &str,
    from_addr: &str,
    received_time: &chrono::DateTime<Local>,
) -> Result<()> {
    let received_header = make_received_header(to_addr, from_addr, received_time);
    let received_header = received_header.as_bytes();
    output
        .write_all(received_header)
        .map_err(|e| anyhow!("Error writing output: {}", e))?;

    let res = process_existing_headers(input, output)?;

    if res.has_date == false {
        let date_str = received_time.to_rfc2822();
        output
            .write_all(format!("Date: {}\n", date_str).as_bytes())
            .map_err(|e| anyhow!("Error writing output: {}", e))?;
    }

    if res.has_from == false {
        output
            .write_all(format!("From: {}\n", from_addr).as_bytes())
            .map_err(|e| anyhow!("Error writing output: {}", e))?;
    }

    // write end-of-headers newline
    output
        .write_all(b"\n")
        .map_err(|e| anyhow!("Error writing output: {}", e))?;

    Ok(())
}

/// Just reads lines from input and writes to output.
pub fn write_body<R: BufRead, W: Write>(input: &mut R, output: &mut W) -> Result<()> {
    let mut buffer = Vec::new();

    loop {
        // read until newline or EOF
        let bytes_read = input
            .read_until(b'\n', &mut buffer)
            .map_err(|e| anyhow!("Error reading input: {}", e))?;

        if bytes_read == 0 {
            break; // reached EOF
        }

        output
            .write_all(&buffer)
            .map_err(|e| anyhow!("Error writing output: {}", e))?;

        // clear for next read
        buffer.clear();
    }

    // flush all buffered data
    output
        .flush()
        .map_err(|e| anyhow!("Error flushing output: {}", e))?;

    Ok(())
}

/// Read headers from input stream, and write a "delivered" version of the
/// message to the output stream (adding appropriate headers).
///
/// The current time is used to get a date-time for the `Received` header.
fn write_message<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    to_addr: &str,
    from_addr: &str,
    received_time: &chrono::DateTime<Local>,
) -> Result<()> {
    write_headers(input, output, &to_addr, &from_addr, &received_time)
        .context("Failed to write headers")?;

    write_body(input, output).context("Failed to write message body")?;

    Ok(())
}

/// validate that a path to a Maildir/new
///
/// - is an absolute path
/// - has `new` as the last component
/// - has `Maildir` as the second-to-last component
///
/// Return Maildir path if valid, or an error message if not.
pub fn parse_maildir_new_path(maildir_new_path: &Path) -> Result<PathBuf> {
    if !maildir_new_path.is_absolute() {
        anyhow::bail!(
            "mailDir path '{:?}' is not an absolute path",
            maildir_new_path
        );
    }

    let components = maildir_new_path.components().collect::<Vec<_>>();

    match components.as_slice() {
        [.., second_to_last, last] => {
            if last.as_os_str() != "new" {
                anyhow::bail!(
                    "mailDir path '{:?}' does not end in 'new'",
                    maildir_new_path
                );
            }
            if second_to_last.as_os_str() != "Maildir" {
                anyhow::bail!(
                    "mailDir path '{:?}' does not have 'Maildir' as the second-to-last component",
                    maildir_new_path
                );
            }
        }
        _ => {
            anyhow::bail!(
                "mailDir path '{:?}' does not end in /Maildir/new",
                maildir_new_path
            );
        }
    }

    let maildir = maildir_new_path.parent().ok_or_else(||
        // actually, if we are here, there is necessarily a parent, but the compiler doesn't
        // know that
        anyhow::anyhow!("mailDir path '{:?}' has no parent", maildir_new_path))?;

    Ok(PathBuf::from(maildir))
}

fn deliver_to_maildir<R: BufRead>(
    input: &mut R,
    from_address: String,
    to_address: String,
    maildir: Maildir,
    received_time: &chrono::DateTime<Local>,
) -> Result<()> {
    let mut mail_mesg_bytes = Vec::<u8>::new();
    write_message(
        input,
        &mut mail_mesg_bytes,
        &to_address,
        &from_address,
        &received_time,
    )
    .context("Couldn't construct delivered message")?;

    let message_id = maildir
        .store_new(&mail_mesg_bytes)
        .map_err(|e| anyhow::anyhow!("Couldn't store message in maildir: {}", e))?;

    log::debug!("Message successfully delivered, with id: {}", message_id);

    Ok(())
}

/// Check if a string is plausible as an email address, in the very loosest sense.
/// We require only that it (a) not be empty and (b) consist only of "graphical" ASCII characters
/// (basically, all letters and digits and punctuation, but not whitespace or control
/// characters).
pub fn is_plausible_string(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_graphic())
}

/// Main logic for the program. Various I/O-type values get injected here as arguments,
/// for easy testing.
///
/// Arguments:
/// - `allowable_program_names`: list of program names we expect to be invoked as (e.g.
///   `sendmail`). We exit with an error if the program name is not one of these.
/// - `ctx`: main context, containing arguments, config path, whether to drop privileges,
///   time we were invoked, etc.
/// - `input`: input stream to read from (stdin, in production)
/// - `output`: optional output stream to write to. Should be `None` in production, but
///    can be used for testing.
///
/// In production, we should _always_ drop privileges; for testing purposes,
/// we might not.
pub fn main<R: BufRead, W: Write>(
    allowable_program_names: &[&str],
    ctx: &MainContext,
    input: &mut R,
    output_opt: Option<&mut W>,
) -> () {
    let prog_name = match ctx.args.as_slice() {
        [prog_name, ..] => prog_name,
        _ => {
            eprintln!("No program name provided.");
            std::process::exit(1);
        }
    };

    // die if not one of the expected program names
    let _prog_name = normalize_prog_name(allowable_program_names, prog_name);

    let cli_options: Command = build_cli();

    let cli_matches = cli_options.get_matches_from(ctx.args.iter());

    // set up logging
    let opt_logfile = cli_matches.get_one::<String>("logfile").cloned();
    match opt_logfile {
        Some(logfile_path) => {
            init_logfile(logfile_path);
        }
        None => {}
    }

    // read config file to get maildir and user name to run as.
    // We never run as root; permanently drop privileges to that user, and if the user
    // _is_ root, fail with an error.
    // Later on - if the specified user can't operate on the Maildir, we'll fail with an
    // error then.

    let config_path = &ctx.config_path;

    log::debug!("Using config file: {:#?}", config_path);

    let config = read_config_ini(config_path).unwrap_or_else(|e| {
        eprintln!("Error reading config file '{}': {}", config_path, e);
        std::process::exit(1);
    });

    log::debug!("Read config: {:?}", config);

    if config.userName == "root" {
        eprintln!("Error: Cannot run as root. Please specify a different user in the config file.");
        std::process::exit(1);
    }

    // drop privileges to the user specified in the config file

    let new_user = User::from_name(&config.userName).map_or_else(
        |err| {
            eprintln!(
                "Error: Couldn't get user '{}' specified in config file: errno was {}",
                config.userName, err
            );
            std::process::exit(1);
        },
        |opt| {
            opt.unwrap_or_else(|| {
                eprintln!(
                    "Error: User '{}' specified in config file is not a valid user",
                    config.userName
                );
                std::process::exit(1);
            })
        },
    );

    match ctx.should_drop_privs {
        PrivilegeOption::NoDropPrivileges => {}
        PrivilegeOption::DropPrivileges => {
            drop_privileges(new_user);
        }
    }

    let from_address = cli_matches
        .get_one::<String>("sender_env")
        .cloned()
        .unwrap_or_else(get_current_user);

    if !is_plausible_string(&from_address) {
        eprintln!(
            "From address '{}' contains non-ASCII, non-printable or whitespace characters, or is zero-length",
            from_address
        );
        std::process::exit(1);
    }

    log::debug!("Using from_address: {:#?}", from_address);

    // if no recipient address is provided, we'll use the name from the config file
    let to_address = cli_matches
        .get_one::<String>("to_address")
        .cloned()
        .unwrap_or_else(|| config.userName.clone());

    if !is_plausible_string(&to_address) {
        eprintln!(
            "Recipient address '{}' contains non-ASCII, non-printable or whitespace characters, or is zero-length",
            to_address
        );
        std::process::exit(1);
    }

    log::debug!("Using to_address: {:#?}", to_address);

    let maildir_new_path = Path::new(&config.mailDir);

    let maildir_path = parse_maildir_new_path(maildir_new_path).unwrap_or_else(|err| {
        eprintln!("Error getting path to maildir: {}", err);
        std::process::exit(1);
    });

    let maildir = Maildir::from(maildir_path.clone());

    match ctx.should_create_maildirs {
        CreateMaildirsOption::CreateMaildirs => {
            maildir.create_dirs().unwrap_or_else(|e| {
                eprintln!(
                    "Error creating Maildir directories at '{:?}': {}",
                    maildir_path, e
                );
                std::process::exit(1);
            });
        }
        CreateMaildirsOption::NoCreateMaildirs => {}
    }

    match (ctx.message_destination, output_opt) {
        (MessageDestination::Maildir, None) => {
            deliver_to_maildir(input, from_address, to_address, maildir, &ctx.received_time)
                .unwrap_or_else(|e| {
                    eprintln!(
                        "Error delivering message to maildir 'new' directiory {:?}: {}",
                        maildir_new_path, e
                    );
                    std::process::exit(1);
                });
            log::debug!("Message successfully delivered to maildir");
        }
        (MessageDestination::OutputStream, Some(output)) => {
            write_message(
                input,
                output,
                &to_address,
                &from_address,
                &ctx.received_time,
            )
            .unwrap_or_else(|e| {
                eprintln!("Error writing message: {}", e);
                std::process::exit(1);
            });
            log::debug!("Message successfully delivered to output stream");
        }
        _ => {
            eprintln!("Error: Invalid combination of message destination and output stream");
            std::process::exit(1);
        }
    }
}

//pub fn bogus_main() {
//    let input = br#"Subject: backupninja: ubuntu2004.localdomain
//To: ggg
//X-Mailer: mail (GNU Mailutils 3.7)
//
//success -- /etc/backup.d/example.sys
//"#;
//
//    let message = MessageParser::default().parse(input).unwrap();
//
//    println!("message: {:#?}", message);
//
//    let new_message = message.clone();
//}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// helper func - standard control flow for all test cases with
    /// `process_existing_headers` as subject under test.
    fn test_headers_helper(input: &[u8], expected_status: HeaderStatus, expected_output: &str) {
        let mut output = Vec::new();
        let result = process_existing_headers(&mut Cursor::new(input), &mut output).unwrap();

        assert_eq!(result, expected_status);
        let output = String::from_utf8(output).unwrap();
        assert_eq!(output, expected_output);
    }

    /// plausible-looking `From:` and `Date:`
    #[test]
    fn test_process_headers_with_from_and_date() {
        let input = b"From: sender@example.com\nDate: Wed, 21 Oct 2020 07:28:00 GMT\n\nBody";
        let expected_status = HeaderStatus {
            has_from: true,
            has_date: true,
        };
        let expected_output = "From: sender@example.com\nDate: Wed, 21 Oct 2020 07:28:00 GMT\n";
        test_headers_helper(input, expected_status, expected_output);
    }

    /// implausible-looking `From:` and `Date:`
    #[test]
    fn test_process_headers_with_implausible_from_and_date() {
        let input = b"From: :?\nDate: ,\n\nBody";
        let expected_status = HeaderStatus {
            has_from: true,
            has_date: true,
        };
        let expected_output = "From: :?\nDate: ,\n";
        test_headers_helper(input, expected_status, expected_output);
    }

    /// `Date:` only
    #[test]
    fn test_process_headers_without_from() {
        let input = b"Date: 21 Oct 2020\n\nBody";
        let expected_status = HeaderStatus {
            has_from: false,
            has_date: true,
        };
        let expected_output = "Date: 21 Oct 2020\n";
        test_headers_helper(input, expected_status, expected_output);
    }

    /// `From:` only
    #[test]
    fn test_process_headers_without_date() {
        let input = b"From: sender@example.com\n\nBody";
        let expected_status = HeaderStatus {
            has_from: true,
            has_date: false,
        };
        let expected_output = "From: sender@example.com\n";
        test_headers_helper(input, expected_status, expected_output);
    }

    /// empty headers
    #[test]
    fn test_process_headers_empty() {
        let input = b"\nBody";
        let expected_status = HeaderStatus {
            has_from: false,
            has_date: false,
        };
        let expected_output = "";
        test_headers_helper(input, expected_status, expected_output);
    }
}
