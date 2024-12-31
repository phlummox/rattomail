#!/usr/bin/env perl


use strict;
use warnings;

# hard-coded values

my $PACKAGE_NAME = 'rattomail';
my $TOML_FILE = 'Cargo.toml';

sub slurp {
  my ($filename) = @_;
  open my $fh, '<', $filename or die "Failed to open $filename for reading: $!";
  local $/; # Enable slurp mode
  my $content = <$fh>;
  close $fh or die "Failed to close $filename: $!";
  return $content;
}

# extract a version from a .toml or similar file.
#
# uses the first occurence of `version = "...some val"` at
# start of line.
#
# args: path to config file
#
# returns: version. or undef on failure
sub extract_version {
  my ($file_path) = @_;
  
  my $file_conts = slurp($file_path);
  
  if ($file_conts =~ /^\s*version\s*=\s*"([^"]+)"/m) {
    return $1;
  }
  
  # if no version found, return undef
  return undef;
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
  my $revision      = $ENV{"RATTOMAIL_REVISION"} // 1;
  my $architecture  = get_architecture();

  # get version from toml file
  my $version = extract_version($TOML_FILE);
  die "Failed to determine version from $TOML_FILE" unless $version;
  my $debver  = "${version}-${revision}";
  my $debfile_name = "$PACKAGE_NAME-$debver-$architecture.deb";
  return $debfile_name;
}

sub get_ver_arch {
  my $architecture  = get_architecture();

  # get version from toml file
  my $version = extract_version($TOML_FILE);
  die "Failed to determine version from $TOML_FILE" unless $version;
  return "$version $architecture";
}

if (@ARGV && $ARGV[0] eq "--ver-arch") {
  # get "$version $arch", e.g. for tgz
  my $ver_arch = get_ver_arch();
  print "$ver_arch\n";
} else {
  my $debfile_name = get_deb_name();
  print "$debfile_name\n";
}

