# Fula Funge - Proof Engine

A digital twin used to simulate, visualize and prove off-chain work.

Run:
```
RUST_LOG="warn,proof_engine=info" cargo run --release -- 0xd13d0307ec662cd6c1c31aeba4a7db9fdfb884d36122c421c06166e5fa91f166
```
Headless Run:
```
RUST_LOG="warn,proof_engine=info" cargo run --features headless --release -- 0xd13d0307ec662cd6c1c31aeba4a7db9fdfb884d36122c421c06166e5fa91f166
```
Where: 0xd13d0307ec662cd6c1c31aeba4a7db9fdfb884d36122c421c06166e5fa91f166 is the seed of the operator responsible for the transactions to make
