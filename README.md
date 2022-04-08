# initial setup:

$ sudo apt install qemu
$ sudo apt install grub-pc-bin # only req'd for WSL

$ rustup override set nightly
$ rustup component add rust-src
$ rustup component add llvm-tools-preview

# to compile:

$ cargo build --target x86_64-os.json
