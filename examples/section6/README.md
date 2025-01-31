# Examples - Section 5
This folder contains the examples as discussed in the paper \[1\].

Specifically, four examples are included:
1. [`section6.3.1.rs`](./section6.3.1.rs) implements an example where some scientist Amy attempts to run a function `entry-count` on a dataset `patients` owned by a hospital, St. Antonius;
2. [`section6.3.2.rs`](./section6.3.2.rs) implements an example where a bookkeeper Bob attempts to run a more complex workflow that dataset `patients` while outsourcing some of his computation;
3. [`section6.3.3_ok.rs`](./section6.3.3_ok.rs) implements BOTH the first two examples such that it runs to completion, showing that the framework is fine working concurrently;
4. [`section6.3.3_crash.rs`](./section6.3.3_crash.rs) implements BOTH the first two examples but with Amy crashing, showing that the framework can still complete the second scenario;
    - However, note that this scenario does NOT terminate. Agents will, after all, keep waiting for Amy's input to it for the first example.
5. [`section6.3.4.rs`](./section6.3.4.rs) implements an example where the St. Antonius internalises some of their policy, even only partially sharing some messages due to sensitivity; and
6. [`section6.3.5.rs`](./section6.3.5.rs) implements an example where the consortium amends the agreement to change what agents must do to justify their actions.


## Running the code
### Dependencies
To run the code, first make sure that you have installed the [Rust](https://rust-lang.org) toolchain.
The easiest way to do so is to use [rustup](https://rustup.rs).
You can download and run the installer on Unix-like operating systems (macOS, Linux) using:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Running examples
Once you have installed Rust, you can simply run each example with the following command:
```bash
cargo run --example XXX --features dataplane,log,serde,slick
```
where `XXX` is the name of one of the sections: `section6-3-1`, `section6-3-2`, `section6-3-3-ok`, `section6-3-3-crash`, `section6-3-4` or `section6-3-5`.

### Using the inspector
The output is, however, quite unreadable by default. As such, you can use the [`inspector`](../../bin/inspector/README.md)-binary to inspect it interatively via a Terminal User Interface (TUI). You can do so by first writing the trace to a file:
```bash
cargo run --example XXX --features dataplane,log,serde,slick -- --output YYY
```
where `YYY` is the path to the file to write to. Subsequently, you can open it in the trace inspector:
```bash
cargo run --package inspector -- --path YYY
```
where `YYY` is the same filepath.

If you are on Unix (e.g., macOS or Linux), you can combine the two commands through the usage of pipes:
```bash
cargo run --example XXX --features dataplane,log,serde,slick | cargo run --package inspector
```


## Reading the code
Reading the code is a little complex, as the code requires some boilerplate to instantiate the holes left by the main JustAct ontology.

The individual agent scripts can be found in [`agents/`](./agents/), each their own file. The files mostly consist of a struct for the agent, some `impl`s, and then finally an `impl Agent<(String, u32), (String, char), str, u64>` that contains its concrete behaviour.

We can make a few observations about these behaviour scripts:
- Despite the generics given to the `Agent`-trait at the `impl` (which defines the types of identifiers, message payloads and timestamps, respectively), the script is written generically over the JustAct ontology. That means that the provided prototype in the crate library is only one possible runtime for these agents.
    - The example to this is that agents need to know how to refer to messages (`(String, u32)` in this case), actions (`(String, char)`) and timestamps (`u64`); also, they need to know how to read message's payloads (`str`).
- Agent can participate in multiple examples, and accordingly, have a [`Script`](./agents/mod.rs)-enum that defines which of the appropriate scripts is executed.
- Further, every agent script is implemented as a series of event listeners, which look for updates in the (Agent's view on) the global state and change it accordingly.
- All the statements that agents exchange can be found in the [`slick/`](./slick/)-folder.
