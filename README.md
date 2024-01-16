# Fula Funge - Proof Engine

A digital twin used to simulate, visualize and prove off-chain work.

Run:

```
RUST_LOG="warn,proof_engine=info" cargo run --release -- ${Operator seed} -- ${Aura account} -- ${Grandpa account}
```

Headless Run:

```
RUST_LOG="warn,proof_engine=info" cargo run --features headless --release -- ${Operator seed} ${Aura account} ${Grandpa account}
```

Where: `Operator seed` represents the seed of the operator responsible for the transactions to make
