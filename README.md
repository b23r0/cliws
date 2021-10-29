# Cliws [![Build Status](https://app.travis-ci.com/b23r0/Cliws.svg?branch=main)](https://app.travis-ci.com/b23r0/Cliws)
Run a process and forwarding stdio to websocket.

# Build & Run

`$> cargo build`
`$> ./target/debug/cliws`

# Usage

You can run a bash and listen port at 8000

`$> ./cliws -p 8000 bash -i`

then connect and get a shell

`$> ./cliws -c ws://127.0.0.1:8000`


# License

MIT.