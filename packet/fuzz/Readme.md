# Fuzz apiformes-packet with cargo fuzz

Note that `cargo fuzz` requires rust nightly, to install cargo fuzz, run:

```sh
cargo install cargo-fuzz
```

Note that you may use the publicly available corpus by executing this command in the current directory:

```sh
git clone https://github.com/cgv6n3qy/packet-corpus corpus
```

To begin fuzzing run:

```sh
cargo fuzz run unstructured_fuzzer
```
