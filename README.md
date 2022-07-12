# Fula Funge - Proof Engine

A digital twin used to simulate, visualize and prove off-chain work.

```
RUST_LOG="warn,fula_funge=info" cargo run --release -- //Alice --pool-id 1000000
```

## Cloud Configuration

- Remove the default bootstrap nodes
```bash
docker exec -it fula-funge_ipfs_1 ipfs bootstrap rm --all
```

- Add the bootstrap node to peer with
```bash
docker exec -it fula-funge_ipfs_1 ipfs bootstrap add /ip4/34.229.178.96/tcp/4001/ipfs/12D3KooWRVbDJXSG3QnKedCFxEkH5UWL2B1weibsBKzP3P9BPDg5
```
