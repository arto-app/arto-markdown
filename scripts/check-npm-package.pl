#!/usr/bin/env perl
# Fail when the npm package `npm publish` would pack from <dir> is missing a
# file its `exports` point at, or the generated theme stylesheet, or carries a
# test. The `files` list in package.json decides what is packed, and nothing
# else notices a file it leaves out until a consumer's import fails.
#
# Usage: check-npm-package.pl <package dir>
use strict;
use warnings;
use JSON::PP qw(decode_json);

my ($dir) = @ARGV;
die "usage: check-npm-package.pl <package dir>\n" unless defined $dir;
chdir $dir or die "$dir: $!\n";

my $manifest = do {
    open my $fh, '<:raw', 'package.json' or die "package.json: $!\n";
    local $/;
    decode_json(<$fh>);
};

my $pack = decode_json(scalar qx(npm pack --dry-run --json --ignore-scripts 2>/dev/null));
die "npm pack failed\n" if $? != 0;
my %packed = map { $_->{path} => 1 } @{ $pack->[0]{files} };

my @failures;
for my $key ( sort keys %{ $manifest->{exports} } ) {
    ( my $target = $manifest->{exports}{$key} ) =~ s{^\./}{};
    if ( $target =~ /\*/ ) {
        ( my $prefix = $target ) =~ s/\*.*//;
        push @failures, "export $key: nothing packed under $prefix"
          unless grep { index( $_, $prefix ) == 0 } keys %packed;
    }
    else {
        push @failures, "export $key: $target is not packed" unless $packed{$target};
    }
}
push @failures, 'style/generated/primer-themes.css is not packed (run `just assets`)'
  unless $packed{'style/generated/primer-themes.css'};
push @failures, map { "$_ is a test and is packed" } grep { /\.test\.ts$/ } sort keys %packed;

if (@failures) {
    print STDERR "$_\n" for @failures;
    exit 1;
}
printf "npm package ok: %d files\n", scalar keys %packed;
