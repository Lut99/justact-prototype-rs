# Examples - Section 5
This folder contains the examples as discussed in the paper \[1\].

Specifically, four examples are included:
1. [`section6.3.1.rs`](./section6.3.1.rs) implements an example where some scientist Amy attempts to run a function `entry-count` on a dataset `patients` owned by a hospital, St. Antonius;
2. [`section6.3.2.rs`](./section6.3.2.rs) implements an example where a bookkeeper Bob attempts to run a more complex workflow that dataset `patients` while outsourcing some of his computation;
3. [`section6.3.4.rs`](./section6.3.4.rs) implements an example where the St. Antonius internalises some of their policy, even only partially sharing some messages due to sensitivity; and
4. [`section6.3.5.rs`](./section6.3.5.rs) implements an example where the consortium amends the agreement to change what agents must do to justify their actions.


## Reading the code
Reading the code is a little complex, as the code requires some boilerplate to instantiate the holes left by the main JustAct ontology.

The individual agent scripts can be found in [`agents/`](./agents/), each their own file. The files mostly consist of a struct for the agent, some `impl`s, and then finally an `impl Agent<(String, u32), (String, char), str, u64>` that contains its concrete behaviour.

We can make a few observations about these behaviour scripts:
- Despite the generics given to the `Agent`-trait at the `impl` (which defines the types of identifiers, message payloads and timestamps, respectively), the script is written generically over the JustAct ontology. That means that the provided prototype in the crate library is only one possible runtime for these agents.
- Agent can participate in multiple examples, and accordingly, have a [`Script`](./agents/mod.rs)-enum that defines which of the appropriate scripts is executed.
- Further, every agent script is implemented as a state machine, where transitions between states occur when the agent receives certain information from other agents and have processed some effect.
- All the statements that agents exchange can be found in the [`slick/`](./slick/)-folder.


## Running the code
You can simply run each example with the following command:
```bash
cargo run --example XXX --features dataplane,log,serde,slick
```
where `XXX` is the name of one of the sections: `section6-3-1`, `section6-3-2`, `section6-3-4` or `section6-3-5`.

The output is, however, quite unreadable by default. As such, you can use the [`inspector`](../../bin/inspector/README.md)-binary to inspect it interatively:
```bash
cargo run --example XXX --features dataplane,log,serde,slick | cargo run --package inspector
```
This will open a Terminal User Interface (TUI) that you can use to browse through the generated traces. Use in combination with the appropriate sections from the paper to see how the runtime emulates the behaviour described there.
