#!/usr/bin/env perl

=pod

=head1 NAME

build-deb.pl - Build a .deb file (or print the filename that will be generated)

=head1 SYNOPSIS

build-deb.pl [options] exe_file

 Options:
   --help           display documentation on standard out
   --print-deb-name print name of .deb file to be generated

=head1 OPTIONS

=over 4

=item B<--help>

Print a help message and exit.

=item B<--print-deb-name>

Print the name of the .deb file to be generated, and exit.
The version number is got by executing C<exe_file --version>.
The architecture is got by running C<uname>.

=back

=head1 DESCRIPTION

If given only an executable file, B<build-deb.pl> will build a .deb file, using a hard-coded
control.in template (look for C<$CONTROL_IN> in the script body).

=head1 PREREQUISITES

B<build-deb.pl> requires that Pandoc (L<https://pandoc.org>) be installed and on the PATH.

=head1 ENVIRONMENT

By default, package revision is set to 1, but can be overridden using
the C<RATTOMAIL_REVISION> environment variable.

=cut

use strict;
use warnings;
use Cwd qw(getcwd abs_path);
use File::Basename;
use File::Copy qw(copy);
use File::Path qw(make_path remove_tree);
use File::Temp qw(tempdir);
use Getopt::Long qw(:config no_auto_abbrev); # user must spell out options in full
use Pod::Usage;

# a few hard-coded values
my $CONTROL_IN = 'debian/control.in';
my $MARKDOWN_MAN = 'doc/rattomail.8.md';
my $PACKAGE_NAME = 'rattomail';

sub slurp {
  my ($filename) = @_;
  open my $fh, '<', $filename or die "Failed to open $filename for reading: $!";
  local $/; # Enable slurp mode
  my $content = <$fh>;
  close $fh or die "Failed to close $filename: $!";
  return $content;
}


# get a suitable system architecture string, suitable
# for use in a .deb control file
sub get_architecture {
  my $arch = `uname -m`;

  ($? == 0) or
    die "Error: Failed to get architecture using 'uname -m'\n";

  chomp($arch);

  if ($arch eq 'x86_64') {
    return "amd64";
  } elsif ($arch eq 'i386' || $arch eq 'i686') {
    return "i386";
  } elsif ($arch eq 'aarch64') {
    return "arm64";
  } else {
    die "Error: Unknown architecture: $arch\n";
  }
}

# query architecture and executable to get .deb file name.
#
# args: path to exe file
#
# returns: a list, (debver, .deb file name), where debver =
# "${version}-${revision}".
#
# Package revision is set to 1 by default, but can be overridden using
# the RATTOMAIL_REVISION env var.
#
sub get_deb_name {
  my ($exe_path) = @_;

  my $revision    = $ENV{"RATTOMAIL_REVISION"} // 1;
  my $architecture = get_architecture();

  my $abs_exe_path = abs_path($exe_path);
  defined $abs_exe_path or
    die "Failed to resolve absolute path for '$abs_exe_path': $!";

  # get version from binary
  my $version = `$abs_exe_path --version`;
  $? == 0 or
    die "Couldn't run '$abs_exe_path --version': $!";
  ($version) = $version =~ /^.*? (\S+)/ if defined $version;
  my $debver  = "${version}-${revision}";

  die "Failed to determine version from $exe_path" unless $version;

  my $debfile_name = "$PACKAGE_NAME-$debver-$architecture.deb";
 
  return ($debver, $debfile_name); 
}

# make a .deb file.
#
# Package revision is set to 1, but can be overridden using
# the REVISION env var.
sub make_deb {
  my ($exe_path, $work_dir) = @_;

  my $architecture = get_architecture();
  my ($debver, $debfile_name) = get_deb_name($exe_path);
  my $exe_name = basename($exe_path);

  my $bin_dir     = File::Spec->catdir($work_dir, "usr", "sbin");
  my $man_dir     = File::Spec->catdir($work_dir, "usr", "share", "man", "man8");
  my $example_dir = File::Spec->catdir($work_dir, "usr", "share", "doc", "$PACKAGE_NAME", "examples");
  my $debian_dir  = File::Spec->catdir($work_dir, "DEBIAN");

  make_path($bin_dir, $man_dir, $example_dir, $debian_dir, {
      chmod => 0755,
  });

  # strip and copy exe
  copy($exe_path, $bin_dir)
    or die "Failed to copy $exe_path to $bin_dir: $!";
  system("strip", "$bin_dir/$exe_name") == 0
    or die "Failed to strip $bin_dir/$exe_name: $!";
  chmod 04755, "$bin_dir/$exe_name" or die "Failed to set setuid bit on $bin_dir/$exe_name: $!";

  # create man page
  system("pandoc -s -t man -f markdown -o $man_dir/$PACKAGE_NAME.8 $MARKDOWN_MAN") == 0
    or die "Pandoc invocation failed: $!";

  # create symlink in bin directory
  my $cwd = getcwd();
  chdir $bin_dir or die "Failed to change directory to $bin_dir: $!";
  # we need to symlink from the _final_ expected dest of rattomail, to the target
  symlink("/usr/sbin/$PACKAGE_NAME", "sendmail") or die "Failed to create symlink 'sendmail': $!";
  chdir $cwd or die "Failed to return to original directory: $!";

  # calculate installed size in kbytes
  my $installed_size = `du -k -s $work_dir`;
  die "Failed to calculate installed size for $work_dir" if $? != 0;
  chomp $installed_size;
  $installed_size =~ s/\s.*//; # strip everything but first field

  # generate the control file
  my $control_out = "$debian_dir/control"; # Path for generated control file

  my $control_conts = slurp($CONTROL_IN);
  $control_conts =~ s/VERSION/$debver/g;
  $control_conts =~ s/ARCHITECTURE/$architecture/g;
  $control_conts =~ s/INSTALLED_SIZE/$installed_size/g;

  open my $out, '>', $control_out or die "Failed to open $control_out for writing: $!";
  print $out $control_conts;
  close $out or die "Failed to close $control_out: $!";

  #system("ls -altd `find $work_dir | sort`");

  # Build the .deb package using fakeroot and dpkg-deb
  my $deb_name = "$PACKAGE_NAME-$debver-$architecture";
  my $deb_file = "$deb_name.deb";
  my $command = "fakeroot dpkg-deb -Zgzip -z9 --build $work_dir $deb_file";
  system($command) == 0
    or die "Failed to build .deb package with command: $command";

  print "Created $deb_name.deb\n";
}

# check that binary file exists, is a file (not symlink),
# is readable, executable, and statically linked.

sub validate_executable {
  my ($exe_path) = @_;

  unless (-e $exe_path) {
      die "Error: file '$exe_path' does not exist.\n";
  }

  unless (-f $exe_path && not (-l $exe_path)) {
      die "Error: '$exe_path' is not a regular file.\n";
  }

  unless (-r $exe_path) {
      die "Error: Cannot read '$exe_path'.\n";
  }

  unless (-x $exe_path) {
      die "Error: '$exe_path' is not executable.\n";
  }

  # check binary is statically linked
  my $file_output = `file $exe_path`;
  die "Failed to run 'file' on $exe_path: $!" if $? != 0;
  $file_output =~ /static.*linked/ or die "Binary $exe_path is not statically linked";
}

sub main_build {
  my ($exe_path) = @_;
  validate_executable( $exe_path );
  print "Executable $exe_path looks ok.\n";

  my $temp_dir = tempdir("./tmp_ratto_build_XXXXXX", CLEANUP => 1);
  make_deb($exe_path, $temp_dir);
}

sub main_print_only {
  my ($exe_path) = @_;
  validate_executable( $exe_path );
  my ($_debver, $debfile_name) = get_deb_name($exe_path);
  print "$debfile_name\n";
}

# cli options
my $print_deb_name = 0;
my $help = 0;

my $res = GetOptions(
  'print-deb-name'  => \$print_deb_name,  # dry-run, just print deb name
  'help|h'          => \$help,            # help flag
);

if (not $res) {
  warn "\nError: Invalid options passed to $0\n\n";
  pod2usage();
}

pod2usage(-verbose => 2, -output => \*STDOUT, -exitval => 0) if $help;

if (@ARGV < 1) {
  warn "Error: No executable provided\n\n";
  pod2usage();
}

my $exe_file = $ARGV[0];

if ($print_deb_name) {
  main_print_only($exe_file);
} else {
  main_build($exe_file);
}


