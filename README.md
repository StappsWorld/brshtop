# Brshtop

A Rust implementation of a Python implementation of Bashtop

At the time of conception, this port was a means of learning Rust for myself. It was an excuse to get to work learning the language and implementing safe code using the built-in Rust compiler tests.

As of right now, this program is a proof of concept. It compiles and runs, but does not accomplish its task of being a true port of BPyTop. Unfortunately, most of the modules need a large overhaul due to their Python-y nature. Also, multi-threading was done unsafely in BPyTop and, thus, doesn't work correctly in this port. 

## Disclaimer

I, in no way, wish to claim that I alone made this project. It is, at its heart, a port of an excellent program that I use every day for myself. Aristocratos developed all the core components in BPyTop, and I simply moved them to Rust with some fixing in the middle to add memory safety. I would say that conceptually, this code is maybe 15%-20% mine in origin, and the rest is Aristocratos' repository.
 
Please see the official release [here](https://github.com/aristocratos/bpytop)


## Usage

This program only works in Linux right now.

In order to compile this program, you will need Cargo, Rust's compiler. It should be included with the Rust language found [here](https://www.rust-lang.org/tools/install)

```bash
git clone https://github.com/StappsWorld/brshtop.git
cd brshtop
cargo build --release
cd ./target/release
./brshtop
```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.