# Data EXchange (DEX) Examples
This folder contains examples of a very, very simple data exchange platform where a user can submit work to a worker to do.

It is a heavily simplified version of Brane that models the following agents:
- [`Driver`](./agents/driver.rs) implements a representative of some user that has a workflow that needs to be executed by the `Worker`;
- [`Worker`](./agents/worker.rs) implements a worker that owns some data and might be willing to do some work if permitted by its `Checker`; and
- [`Checker`](./agents/checker.rs) implements a checker that will decide what the `Worker` is allowed to do.

This example serves to show some more realistic examples of agent behaviour implementations. In particular, every agent works on \*any\* workflow, where the specific workflow differs per example. Currently:
1. 


## Running the code
You can simply run each example with the following command:
```bash
cargo run --example XXX --features dataplane,log,serde,slick
```
where `XXX` is the name of one of the examples: TODO.

The output is, however, quite unreadable by default. As such, you can use the [`inspector`](../../bin/inspector/README.md)-binary to inspect it interatively:
```bash
cargo run --example XXX --features dataplane,log,serde,slick | cargo run --package inspector
```
This will open a Terminal User Interface (TUI) that you can use to browse through the generated traces.
