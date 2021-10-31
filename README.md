# Cliws [![Build Status](https://app.travis-ci.com/b23r0/Cliws.svg?branch=main)](https://app.travis-ci.com/b23r0/Cliws)
Spawn process IO to websocket and support full PTY.

# Features

* Any process IO through Websocket
* Full pty support: VIM, SSH, readline

# Build & Run

`$> cargo build`

`$> ./target/debug/cliws`

# Usage

You can run a bash and listen port at 8000

`$> ./cliws -p 8000 bash -i`

then connect and get a comfortable shell.

`$> ./cliws -c ws://127.0.0.1:8000`

# Example

![image]( https://github.com/b23r0/Cliws/blob/main/example/cliws-vim.gif)

# License

MIT.
