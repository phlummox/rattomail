#!/usr/bin/env perl

# "end-to-end"-style test of rattomail.
#
# - builds a Docker container based on Ubuntu focal with bsd-mailx
#   installed
# - runs container, installs built rattomail .deb file, sends a test email
# - examines Maildir to see if mail was delivered

# prerequisites:
#
# - requires Email::Simple. On debian derivatives, install it with
#   `sudo apt-get install libemail-simple-perl`.

use strict;
use warnings;

use Cwd;
use Cwd 'abs_path';
use Email::Simple;
use File::Find;
use File::Temp qw(tempdir);
use File::Basename;
use Time::HiRes qw(sleep);
use Test::More;

# use a BEGIN block so we print our plan before any modules loaded
BEGIN { plan tests => 7 }

my $TEST_DIR = "docker-tests";

# some globally available vars..

my $container_id;
my $cwd = getcwd();
my $debfile_name;

####
#
# helper subs

sub slurp {
  my ($filename) = @_;
  open my $fh, '<', $filename or die "Failed to open $filename for reading: $!";
  local $/; # Enable slurp mode
  my $content = <$fh>;
  close $fh or die "Failed to close $filename: $!";
  return $content;
}


# build image using provided dockerfile and makefile

sub build_docker_image {
  diag("Building container image");
  chdir "${TEST_DIR}/image"
    or die "Failed to change directory to $TEST_DIR/image: $!";
  system('make build') == 0
    or die "Failed to run 'make build': $!";
  chdir $cwd or die "Failed to change back to original directory: $!";
}

# check file exists and is readable
sub validate_file {
  my ($file) = @_;

  unless (-e $file) {
      die "Error: file '$file' does not exist.\n";
  }

  unless (-f $file && not (-l $file)) {
      die "Error: '$file' is not a regular file.\n";
  }

  unless (-r $file) {
      die "Error: file '$file' does not exist.\n";
  }
}

# run docker container in background.
#
# args: path to deb file
#
# returns: container id
sub run_docker_container {
  my ($debfile_path) = @_;

  validate_file($debfile_path);
  $debfile_path = abs_path($debfile_path);
  defined $debfile_path or
    die "Failed to resolve absolute path for '$debfile_path': $!";

  my $conffile_path = "$TEST_DIR/data/attomail.conf";

  validate_file($conffile_path);
  $conffile_path = abs_path($conffile_path);
  defined $conffile_path or
    die "Failed to resolve absolute path for '$conffile_path': $!";

  print "Running Docker container in background...\n";
  my $docker_cmd = "docker -D run --rm --detach " .
                   " -v $cwd:/work " .
                   " -v $conffile_path:/etc/attomail.conf " .
                   " -v $debfile_path:/tmp/$debfile_name " .
                   " --workdir /work " .
                   " phlummox/test-rattomail:0.1 tail -f /dev/null";
  $container_id = `$docker_cmd`;
  $? == 0 or
    die "Failed to start Docker container: $!";
  chomp($container_id);

  return $container_id;
}

# find all regular files in a directory tree

sub find_file {
  my ($dir_path) = @_;
  my @files = `find $dir_path -type f`;
  $? == 0 or
    die "Couldn't run 'find $dir_path -type f': $!";

  chomp @files;
  
  return @files;
}

###
# main

if (@ARGV < 1) {
    die "Error: No .deb file provided\n";
}

my $debfile_path = $ARGV[0];
$debfile_name = basename($debfile_path);

build_docker_image();
$container_id = run_docker_container($debfile_path);
sleep(0.5);

# clean up container when we exit
END {
  if (defined $container_id) {
    system("docker stop -t 0 $container_id") == 0
      or warn "Failed to stop docker container $container_id: $!\n";
  }
}

# send an email using `mailx` within container

diag("Installing rattomail and running mailx inside Docker container...");

my $exec_cmd = "docker exec $container_id sh -c 'sudo apt install -y /tmp/$debfile_name && echo wobble | mailx -s test foo\@bar.com'";
system($exec_cmd) == 0
  or die "Failed to run 'mailx' in container: $!";

# test contents of maildir - copy from container to a temp dir

my $temp_dir = tempdir("./tmp_ratto_test_XXXXXX", CLEANUP => 1);

my $copy_cmd = "docker cp $container_id:/home/user/Maildir $temp_dir";
system($copy_cmd) == 0
  or die "Failed to copy Maildir from container: $!";

my @files = find_file("$temp_dir");

is(scalar @files, 1, 'Should be exactly 1 file in Maildir, found ' . scalar @files);

my $mailfile = $files[0];

my $mail_conts = slurp($mailfile);

my $email = Email::Simple->new($mail_conts);

# Check headers and body are as expected

my @received = $email->header("Received");
is(scalar @received, 1, 'Should be exactly 1 received header in email, found ' . scalar @received);
my $pattern = qr/rattomail.*?;/;
like($received[0], $pattern, "'received' contains 'rattomail' and semicolon");

my $to = $email->header("To");
is($to, 'foo@bar.com', "'To' matches the expected value");

my $from = $email->header("From");
# by default, the user invoking sendmail will be used
is($from, 'user', "'From' matches the expected value");

my $subject = $email->header("Subject");
is($subject, 'test', "'Subject' matches the expected value");

my $body = $email->body;
is($body, "wobble\n", "Email body matches the expected value");

