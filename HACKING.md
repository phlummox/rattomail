
## maildir format

We trust the maildir crate to sensibly create unique names in Maildir/new.

## unit and integration testing

We inject a lot of IO-dependent context needed for main (e.g. command-line args, path to
config file, time program was invoked) plus key policy decisions (whether to drop
privileges) as `rattomail::MainContext`. This makes the main routine easier to test.

The package defines multiple executables -- one of these is `bogus_rattomail`, which
is identical to the normal executable, except (a) it doesn't drop privileges (so we can
easily run it as any user) and (b) it allows `bogus_rattomail` as an allowable program
name.

## building static executables

standard way is apparently to use musl as the libc.

So one easy approach is to use an alpine-based docker image - e.g. `rust:1.83.0-alpine3.21`

While developing, it's handy to have a persistent volume for ~/.cargo etc.

something like

```
docker -D run --rm -it --name my-ctr -v rust_home_dir:/home/user my/image:0.1
```

## end-to-end testing

**`docker-tests/image`** contains a Dockerfile for an Ubuntu container with `bsd-mailx`
installed, a particularly ill-behaved mail program. (It invokes `sendmail` with very few
args, and uses execve to set the invoked program's `argv[0]` to `send-mail`, for some bizarre reason.)
The container also has installed a "fake" MTA (Mail Transfer Agent) package, mta-dummy,
so that bsd-mailx's dependencies are satisfied.

To build, cd to the directory and `make build`; you can then run the container with, e.g.
`docker run phlummox/test-rattomail:0.1`.

<!--
  vim: tw=90 :
-->
