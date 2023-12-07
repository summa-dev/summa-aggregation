# Summa Aggregation

Summa Aggregation is a scalable solution specifically designed to accelerate the process of building Merkle sum trees. It addresses the time-intensive challenge of constructing these trees by enabling efficient scaling through parallelization and distributed computation across multiple machines.

## Running test

Tests can be run using the following command:

```bash
cargo test --release
```

Note: The Worker will run locally and uses port 4000 as the default for its server.
Please ensure that this port is not already in use to avoid errors.

## Running Additional Tests Involving Docker and Docker Swarm

For additional tests involving Docker and Docker Swarm mode, the presence of the "summadev/summa-aggregation-mini-tree" image in the local Docker registry is required.


### Building the docker image

Build the image using the following command:

```bash
docker build . -t summadev/summa-aggregation-mini-tree
```

### Downloading the Docker Image

Alternatively, the image can be downloaded from Docker Hub:

```bash
docker pull summadev/summa-aggregation-mini-tree
```

### Testing with LocalSpawner

The following command runs an additional test case using the LocalSpawner, which spawns worker containers in the local Docker environment. This extra test case involves running two containers during the testing process:


```bash
cargo test --features docker
```

### Testing with CloudSpawner

For Summa-Aggregation, it's necessary to prepare a distributed environment where Workers can operate on remote machines, referred to as 'Nodes'. For guidance on setting up swarm nodes, please see [Getting Started with swarm mode](https://docs.docker.com/engine/swarm/swarm-tutorial)

When the Docker environment is running successfully in Swarm mode, an additional test case that spawns workers on Swarm nodes using the `CloudSpawner` can be run:

```bash
cargo test --features docker-swarm
```

It is critical to ensure that the Docker Swarm includes at least one node connected to the manager node. Additionally, each worker node in the swarm must have the "summadev/summa-aggregation-mini-tree" image in its Docker registry. Without this image on nodes connected to the manager node, spawning workers on that node is not possible.

## Summa Aggregation Example

This example demonstrates the setup and operation of a distributed environment using Summa Aggregation, including the initialization of round and generating inclusion proof. A notable aspect of this demonstration is how the AggregationMerkleSumTree can produce the generation of inclusion proofs, similarly to the MerkleSumTree.

### 1. Setup Distributed Environment

Custodians can leverage any cloud infrastructure to establish worker nodes. In this example, we use two local servers running mini-tree services as workers, rather than deploying worker containers on remote nodes.

Key steps:

- **Spawning Worker Nodes**: Two local servers are spawned, each running a mini-tree service.

- **Worker URLs**: It is crucial to ensure the number of worker URLs matches the number of executors. In this example, we use `127.0.0.1:4000` and `127.0.0.1:4001`.

### 2. Initialize the Round with Aggregation Merkle Sum Tree

Initiating the round with an `AggregationMerkleSumTree` is a key step after setting up the distributed environment with worker nodes. This process involves the `Orchestrator` and the `Round`.

- **Orchestrator and AggregationMerkleSumTree**: The `Orchestrator` is initialized with the `CloudSpawner` and paths to the CSV files containing entry data. It uses this information to generate the `AggregationMerkleSumTree`, which forms the basis for the round's operations.

- **Round Initialization**: Subsequently, the `Round` is initialized using the aggregation merkle sum tree. The `Round` is integral for interactions with the Summa contract and relies on the setup performed by the `Orchestrator`.

### 3. Interact with the Summa Contract and Generate Proof of Inclusion

The actual example only shows the creation of an inclusion proof.

For detailed information on interaction patterns similar to those in the `summa-backend` example, refer to the ['summa_solvency_flow'](https://github.com/summa-dev/summa-solvency/blob/master/backend/examples/summa_solvency_flow.rs).

### Example Execution

Run the example using the following command:

```bash
cargo run --release --example aggregation_flow
```
