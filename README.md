# Cliws [![Build Status](https://img.shields.io/github/workflow/status/b23r0/cliws/Rust)](https://app.travis-ci.com/b23r0/Cliws) [![ChatOnDiscord](https://img.shields.io/badge/chat-on%20discord-blue)](https://discord.gg/ZKtYMvDFN4) [![LastCommit](https://img.shields.io/github/last-commit/b23r0/cliws)](https://github.com/b23r0/Cliws/) [![Crate](https://img.shields.io/crates/v/cliws)](https://crates.io/crates/cliws)
Lightweight interactive bind/reverse PTY shell implementation by Rust.

# Features

* WebSocket
* Full pty support: VIM, SSH, readline, Ctrl+X
* Auto set terminal window size.
* Reverse connection / Bind port
* Support Win10+(Windows Server 2019+) & Linux & BSD & OSX

# Build & Run

`$> cargo build --release`

`$> ./target/release/cliws`

# Installation

`$> cargo install cliws`

# Usage

## Bind Mode

You can run a bash and listen port at 8000

`$> ./cliws -p 8000 bash -i`

then connect and get a comfortable shell.

`$> ./cliws -c ws://127.0.0.1:8000`

## Reverse Mode

First listen a port wait for shell

`$> ./cliws -l 8000`

then build a reverse connection

`$> ./cliws -r ws://127.0.0.1:8000 bash -i`

# Example

## Linux

![image]( https://github.com/b23r0/Cliws/blob/main/example/cliws-vim.gif)

## Windows(Reverse Mode)

![image]( https://github.com/b23r0/Cliws/blob/main/example/cliws-windows.gif)

# Invalid Characters

In Windows(Windows Terminal), the default `CodePage` encoding is UTF-8. When encountering the target of other language operating systems, invalid characters may occur. You can try the following methods to solve it.

Open Regedit and modified `[Machine]\HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\Nls\CodePage\OEMCP` value is 65001(DEC).

# Reference

* https://github.com/t57root/amcsh

* https://github.com/philippkeller/rexpect

* https://github.com/zhiburt/conpty

* https://securityonline.info/conptyshell-interactive-reverse-shell/
