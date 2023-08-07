# Fula Funge - Proof Engine

A digital twin used to simulate, visualize and prove off-chain work.

[//]: # (SBP-M1 review: based on docker-compose.yaml, Alice and Bob are validators of the chain. Assuming that the below will be run by an end user, better to use another dev account such as Ferdie implying a standard user)
```
RUST_LOG="warn,proof_engine=info" cargo run --release -- //Alice --pool-id 1000000 --asset-id 1000000
```

[//]: # (SBP-M1 review: add information about using docker-compose)