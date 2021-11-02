# Cliws [![Build Status](https://app.travis-ci.com/b23r0/Cliws.svg?branch=main)](https://app.travis-ci.com/b23r0/Cliws) (https://img.shields.io/badge/chat-on%20discord-blue)](https://discord.gg/ZKtYMvDFN4)
Spawn process IO to websocket with full PTY support.

# Features

* Any process IO through Websocket
* Full pty support: VIM, SSH, readline, Ctrl+X
* Auto set terminal window size.
* Reverse connection / Bind port

# Build & Run

`$> cargo build --release`

`$> ./target/release/cliws`

# Usage

## Direct

You can run a bash and listen port at 8000

`$> ./cliws -p 8000 bash -i`

then connect and get a comfortable shell.

`$> ./cliws -c ws://127.0.0.1:8000`

## Reverse

First listen a port wait for shell

`$> ./cliws -l 8000`

then build a reverse connection

`$> ./cliws -r ws://127.0.0.1:8000 bash -i`

# Example

![image]( https://github.com/b23r0/Cliws/blob/main/example/cliws-vim.gif)

# Reference

* https://github.com/t57root/amcsh

* https://github.com/philippkeller/rexpect
