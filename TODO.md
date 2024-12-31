
- Allow logging using syslog? or /dev/stdout?

- Maybe cache the docker image used by docker-test.pl?

  Can build it whenever the Dockerfile changes and push it to github's repos.

- probably is safer to use capabilities instead of setuid.

  but no-one does, 'cos it's a pain.
