# Brshtop

A Rust implementation of a C++ implementation of a Python implementation of Bashtop (Version 2!)

At the time of conception, this port was a means of learning Rust for myself. It was an excuse to get to work learning the language and implementing safe code using the built-in Rust compiler tests.

As of right now, we're working on rewriting BRShtop as a whole and seeing if we can rewrite my old code better with bettwe concurrency and less borrow checker issues. 🙂

## Disclaimer

I, in no way, wish to claim that I alone made this project. I have an excellent team of folks helping me out with this project and, at its heart, it's a port of an excellent program that I use every day for myself. Aristocratos developed all the core components in BPyTop.
 
Please see the official release [here](https://github.com/aristocratos/bpytop)


## Usage

As we're writing this app, we will post more usage instructions, but the general rundown is here:

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