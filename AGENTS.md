This is an ongoing Rust rewrite of the original `ocx` application prototype written
in nushell.

Refences:
- local clone of ocx repo: `.mem/master/ref/repos/palekiwi-labs/ocx`
- preliminary specification: `.mem/master/spec/ocx-spec.md`

The rewrite is not intended to be a 1:1 clone, for example only the "nix flow"
will be implemented (nix daemon and dev containers).

Load the "tdd" skill - we will apply TDD techniques as far as possible in this rewrite.

Support the following platforms only:
- Linux x86
- MacOS arm
