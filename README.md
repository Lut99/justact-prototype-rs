# justact-prototype-rs: A Prototype JustAct Implementation
A prototype that uses the JustAct[\[1,2\]](#references) framework to exchange policy information between actors.

This crate builds on the ontology defined in the [justact-rs](https://github.com/Lut99/justact-rs)-crate.

> If you are here from the paper, welcome! Please take a look at the [paper examples](./examples/section6/README.md) for more information on how to reproduce paper results.
> 
> Table of contents
> - If you're here looking for the prototype, you're at the right place.
> - If you're here looking for the ontology, see [`lib/justact/`](./lib/justact/README.md).
> - If you're here looking for the Slick interpreter, see [`lib/slick/`](./lib/slick/README.md).
> - If you're here looking to run benchmarks, see [below](#running-benchmarks).


## Contributions
This crate mainly contributes the [`runtime::Runtime`](./src/runtime.rs)-struct, which implements a simple runtime for JustAct-compatible agents.
It works by simply polling the given agents (and a single synchronizer) until all of them have reported `Poll::Ready` (i.e., they terminated).
The JustAct sets themselves are implemented purely in-memory.
For more details on the specifics, refer to the extended version of the paper[\[2\]](#references).

The output of using the runtime produces a trace of [events](./src/auditing.rs), which encodes an audit log.
The provided [trace inspector](./bin/inspector/README.md) is capable of interactively exploring these. We recommend you to examine it for more information.

### Examples
The library is bundled with a few examples to show its usage:
- [examples/section6/](./examples/section6/README.md) are the most elaborate, and implement the examples from the extended version of the paper[\[2\]](#references). See its README for more information.
- [examples/invalid/](./examples/invalid/README.md) show a few miscellaneous and simpler examples for when not all agents are nicely behaved. See its README for more information.

### Policy languages
The prototype supports multiple policy languages. Currently, the following are supported:
- [$Datalog^\neg$](https://github.com/Lut99/datalog-rs); and
- [Slick](https://github.com/sirkibsirkib/slick).

Note that adaptors for both are implemented in [`src/policy/`](./src/policy/) to make them compatible with the main ontology.

### Miscellaneous
This crate also provides some helpers for agents using it. Specifically, it contributes:
- A [dataplane](./src/dataplane.rs) that models "real world" effects of agents. It is implemented as a simple, in-memory variable store from/to which agents collaboratively read/write;
- An [event handler](./src/events.rs) that provides agents with an "event handler"-like interface to reading JustAct sets. The agents in the [paper examples](./examples/section6/README.md) make use of this.


## Usage
You can depend on the prototype for use in your own projects. Typically, you would do so to implement your own agents.

To do so, simply add it to your Cargo.toml file:
```toml
[dependencies]
justact-prototype = { git = "https://github.com/Lut99/justact-prototype-rs" }
```
You can optionally commit yourself to a specific tag. For example:
```toml
[dependencies]
justact-prototype = { git = "https://github.com/Lut99/justact-prototype-rs", tag = "v1.0.0" }
```

### Running examples
More information for running examples can be found by checking that example's README in [the examples folder](./examples/). In general, you can use `cargo run`:
```sh
cargo run --example XXX
```
where `XXX` is the name of the example.

Sometimes, examples may require specific features to be enabled. Cargo will tell you if that's the case. Generally, you need to:
```sh
cargo run --example XXX --features YYY
```
where `YYY` is a comma-separated list of features.

### Running the inspector
You can run the inspector by asking Cargo to build and run it for you:
```sh
cargo run --package inspector
```
By default, it will attempt to read a trace from stdin. You can instead read from a file using the `-p`/`--path`-option:
```sh
cargo run --package inspector -- --path XXX
```
where `XXX` is the path to the trace file in question.

In addition, on Unix (macOS/Linux), you can also input the trace from an example directly using pipes:
```sh
cargo run --example XXX | cargo run --package inspector
```

### Running benchmarks
Finally, you can also run a benchmark of (almost) all examples from the paper.
To do so, use the [`benchmark.py`](./benchmark.py) script:
```sh
python3 benchmark.py
```
You can use `python3 benchmark.py --help` for more information on how to use it.


## Features
This crate supports the following features:
- `all-lang`: Enables all supported policy languages (`datalog`)
- `datalog`: Enables implementations for the $Datalog^\neg$ language.
- `slick`: Enables implementations for the Slick language.
- `dataplane`: Enables a simple dataplane implementation as a key/value store.
- `lang-macros`: Enables language macros. In particular, enables the `datalog!()` embedded DSL macro.
- `log`: Adds support for the [`log`](https://github.com/rust-lang/log)-crate.
- `serde`: Adds support for the [`serde`](https://github.com/serde-rs/serde)-crate.


## Contribution
Contributions to this crate are welcome! If you have any suggestions, fixes or ideas, please feel free to [leave an issue](/Lut99/justact-prototype-rs/issues) or [create a pull request](/Lut99/justact-prototype-rs/pulls).


## License
This project is licensed under Apache 2.0. See [LICENSE](./LICENSE) for more details.


## References
\[1\] Esterhuyse, C.A., Müller, T., van Binsbergen, L.T. (2024). _JustAct: Actions Universally Justified by Partial Dynamic Policies._ In: Castiglioni, V., Francalanza, A. (eds) Formal Techniques for Distributed Objects, Components, and Systems. FORTE 2024. Lecture Notes in Computer Science, vol 14678. Springer, Cham. <https://doi.org/10.1007/978-3-031-62645-6_4>
\[2\] TODO
