# rattomail

Minimal mail delivery agent (MDA) for local mail with Maildir support - a Rust re-implementation of
[femtomail](https://git.lekensteyn.nl/femtomail/), and a drop-in replacement for
[attomail](https://github.com/phlummox/attomail).

## purpose

Acts as a minimal (local delivery only) mail
delivery agent, or MDA, delivering mail to a local Maildir-format directory.
Is compatible with the traditional Unix `sendmail` command.
Handy when you don't want to install an MTA (Mail Transfer Agent) or
fuller-featured MDA - you just want a program which accepts 
`sendmail`-style delivery of messages from local programs, and dumps them
somewhere you can read them. 

It is a port of [femtomail](<https://git.lekensteyn.nl/femtomail/>) to Haskell.
(See this [StackExchange](http://unix.stackexchange.com/questions/82093/minimal-mta-that-delivers-mail-locally-for-cron) posting for femtomail's inception.)

## configuration

By default, uses `/etc/attomail.conf` as a configuration file.

`/etc/attomail.conf` needs to contain two lines, specifying the path to a Maildir/new
directory where messages should be delivered, and the user that owns that directory.

e.g.:

    mailDir = /path/to/my/home/dir/Maildir/new 
    userName = myuserid

## building and installing

If you have a stable rust toolchain installed, you should be able to build
with `cargo build --release`.

Copy `target/release/rattomail` to `/usr/sbin/sendmail`, set the owner to `root`,
and set the setuid permissions bit on the executable. (Needed because `rattomail` changes to
the user specified in `/etc/attomail.conf` when delivering mail.)

If you have GNU make installed, `sudo make install` will do this for you.

If you use a Debian derivative, .deb files are available from the "Releases" page.

## command-line arguments

Usage: 

*   `attomail rattomail [OPTIONS] [RECIPIENT]`

No options are mandatory, and neither is a recipient. By default, `rattomail` will read an
email message on standard input, and deliver it to the Maildir/new directory specified in
`/etc/attomail.conf`.

Many of the options exist only for compatibility with traditional `sendmail`, and are ignored.

The few options that do have an actual effect are:

    --version         Print the program version
    -f <ADDRESS>      Set the sender (from) envelope address. If not specified, the
                      current user is used. Must not contain non-ASCII, whitespace or
                      non-printable characters.
    -b <MODE>         -bm: Read input from stdin (default). Any other mode is an error.
    -X <LOGFILE>      Log debugging messages to a file. The only valid values are
                      /dev/stderr and '-', which has the same meaning.
    -h, --help        Print help

## testing the installation

-   If you have a `mail` program installed, just use that for testing the
    installation. Messages to any address at all, local or remote, should go
    to the Maildir directory specified.

-   Alternatively:

    ~~~
    $ cat | sendmail a@b.com << EOF 
    To: someone@somewhere
    Subject: mysubject
    
    some body
    EOF
    ~~~

    *`<ctrl-d>`*

## portability

Probably won't work on anything but Linux systems.


