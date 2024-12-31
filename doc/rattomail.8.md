---
title: RATTOMAIL(8) Simple sendmail replacement
author: phlummox
---

# NAME

**rattomail** -- read a message on stdin and deliver it to a Maildir-format directory

# SYNOPSIS

**rattomail** [*OPTIONS*...] [*RECIPIENT*]

# DESCRIPTION

**rattomail** acts as a minimal (local delivery only) mail delivery agent, or
MDA, delivering mail to a local Maildir-format directory. If the required
directories do not exist, they are created.

**rattomail** reads a message from standard input until EOF, and delivers the
incoming message to a Maildir-format directory, specified in
*/etc/rattomail.conf*.

A recipient can be specified on the command-line, but is not required, since
all mail will be delivered to the specified Maildir directory.

# OPTIONS

The options are as follows:

**\-\-version**

:   Print the program version and exit.

**-f** *sender*

:   Set the sender envelope address. If not specified, the current user is
    used. Must not contain non-ASCII, whitespace or non-printable characters.

**-b** *MODE*

:   If *MODE* is *m*: read input from stdin (the default behaviour). Supplying
    any other mode is an error.

**-X** *LOGFILE*

:   Log debugging messages to a file. The only valid values are `/dev/stderr` and
    `-`, which has the same meaning.

**-h**, **\-\-help**

:   Print help.

To maintain compatibility with traditional `sendmail`, the following options are also accepted, but have no effect:

**-i** \
**-n**     \
**-t**     \
**-o** *o* \
**-p** *p* \
**-q** *q* \
**-r** *r* \
**-v** *v* \
**-B** *B* \
**-C** *C* \
**-F** *F* \
**-N** *N* \
**-O** *O* \
**-R** *R* \
**-U** *U* \
**-V** *V*


# SETUP

After installation: copy */usr/share/doc/rattomail/examples/attomail.conf* to
*/etc/attomail.conf*, and edit it as required.

Example contents:

```
mailDir = /path/to/my/home/dir/Maildir/new
userName = myuserid
```

'mailDir' is the path to a 'Maildir'-style folder where mail should be delivered;
'userName' is the userid to change to when delivering mail. (Normally, the
owner of the mail folder.)

# USAGE

```
$ cat | sendmail a@b.com << EOF
To: someone@somewhere
Subject: mysubject

some body
EOF
```

# EXIT STATUS

**rattomail** exits with 0 on success, and 1 if an error occurs.

# FILES

**/etc/attomail.conf**

:   Configuration file, specifying `mailDir` and `userName`.

# COPYRIGHT

Copyright 2024, Phlummox. Licensed under the Simplified BSD License, see
<https://opensource.org/license/bsd-2-clause>.

# CREDITS

**rattomail** is a port of `femtomail` <https://git.lekensteyn.nl/femtomail/>
to Rust, and replaces **attomail** <https://github.com/phlummox/attomail>.

