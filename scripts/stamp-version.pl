#!/usr/bin/env perl
# Stamps a release version over the 0.0.0 placeholder in Cargo.toml, in the
# Cargo.lock entries of the workspace's own packages (the ones without a
# `source`), so `--locked` still holds, and in the npm package's
# package.json, so the crate and the package go out at one version.
#
# The release workflow runs this before it builds, and `just ci` runs it on a
# copy, so a placeholder it no longer finds fails a pull request rather than a
# release. A file with nothing to stamp is an error for the same reason.
# Lines may end in CRLF: a Windows checkout without eol=lf has them.
use strict;
use warnings;

my ( $version, $dir ) = @ARGV;
die "usage: stamp-version.pl <version> [<dir>]\n" unless defined $version;
die "not a semver version: $version\n"
  unless $version =~ /\A[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?\z/;
$dir //= '.';

my %stamp = (
    'Cargo.toml' => sub { $_[0] =~ s/(\bversion\s*=\s*)"0\.0\.0"/$1"$version"/g },
    'Cargo.lock' =>
      sub { $_[0] =~ s/(\r?\nversion = )"0\.0\.0"(?=\r?\n(?!source))/$1"$version"/g },
    'packages/markdown/package.json' =>
      sub { $_[0] =~ s/(\r?\n  "version": )"0\.0\.0"/$1"$version"/g },
);

for my $file ( sort keys %stamp ) {
    my $path = "$dir/$file";
    open my $in, '<:raw', $path or die "$path: $!\n";
    my $text = do { local $/; <$in> };
    close $in;

    my $count = $stamp{$file}->($text);
    die "$path: no 0.0.0 placeholder to stamp\n" unless $count;

    open my $out, '>:raw', $path or die "$path: $!\n";
    print {$out} $text;
    close $out;
}
