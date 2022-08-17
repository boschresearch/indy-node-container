# indy_load_generator
This project aims to provide a load generator for identity networks based on [indy](https://github.com/hyperledger/indy-node).
The code heavily depends on [indy-vdr](https://github.com/hyperledger/indy-vdr) and its features. The long-term goal is to provide a configurable tool to simulate different usage-scenarios and help with stability tests for the node implementation.

# Usage

The project can be built using standard rust tooling: `cargo build` or executed via `cargo run`.
Logging can be enabled using the environment variable`RUST_LOG`, e.g. `RUST_LOG=info ./indy_load_generator`

### CLI options
```
USAGE:
    indy_load_generator [OPTIONS]

OPTIONS:
    -d, --duration <DURATION>       Time to run for in seconds, if not provided will run until
                                    SIGTERM/SIGINT is received
    -g, --genesis <GENESIS_FILE>    Path to Pool transaction genesis file [default:
                                    ./pool_transactions_genesis]
    -h, --help                      Print help information
    -r, --reads <READS>             Reads per write, Not yet implemented [default: 0]
    -s, --seed <SEED>               Seed to sign transactions with [default:
                                    000000000000000000000000Trustee1]
    -t, --threads <THREADS>         Parallel worker threads, defaults to number of logical cores
                                    available if not given
    -V, --version                   Print version information
```
