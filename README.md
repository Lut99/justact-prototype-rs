# justact-prototype-rs: A Prototype JustAct Implementation
A prototype that uses the JustAct[\[1\]](#references) framework to exchange policy information between actors.

This crate builds on the ontology defined in the [justact-rs](https://github.com/Lut99/justact-rs)-crate.


## Execution
This environment performs simple step-wise execution of all agents.

Specifically, it implements an executor that:
1. Executes the `poll()` method on all its `Agent`s once;
2. Runs an audit on all published `Action`s, reporting their validity to the user;
3. Removes any `Agent`s that have reported `AgentPoll::Dead`.
4. Goes to 1 as long as there is at least one agent left.


## Policy languages
The prototype supports multiple policy languages. Currently, the following are supported:
- [$Datalog^\neg$](https://github.com/Lut99/datalog-rs).


## Usage
To run the prototype, choose the example that you want to run. For example:
```sh
cargo run --example paper1
```

Note that examples may require specific features. If so, cargo will error and tell you which ones you need to specify. For example:
```sh
cargo run --example paper1 --features datalog
```


## Features
This crate supports the following features:
- `all-lang`: Enables all supported policy languages (`datalog`)
- `datalog`: Enables implementations for the $Datalog^\neg$ language.


## Contribution
Contributions to this crate are welcome! If you have any suggestions, fixes or ideas, please feel free to [leave an issue](/Lut99/justact-prototype-rs/issues) or [create a pull request](/Lut99/justact-prototype-rs/pulls).


## License
This project is licensed under Apache 2.0. See [LICENSE](./LICENSE) for more details.


## References
\[1\] Esterhuyse, C.A., Müller, T., van Binsbergen, L.T. (2024). _JustAct: Actions Universally Justified by Partial Dynamic Policies._ In: Castiglioni, V., Francalanza, A. (eds) Formal Techniques for Distributed Objects, Components, and Systems. FORTE 2024. Lecture Notes in Computer Science, vol 14678. Springer, Cham. <https://doi.org/10.1007/978-3-031-62645-6_4>
