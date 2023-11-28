# Summa Aggregation

Summa Aggregation is focused on optimizing the generation of Merkle sum tree, a task identified as the primary time-consuming process in Summa benchmarks. Our goal is to significantly reduce the time required to generate these tree by leveraging parallelization across multiple machines.

The system features `AggregationMerkleSumTree`, a specialized component designed for the efficient construction of complete tree from smaller, aggregated structures known as `mini-tree`. This approach not only speeds up the tree-building process but also enhances scalability and performance in large-scale data environments.

## Orchestrator

The Orchestrator in the Summa Aggregation serves as the central management component, coordinating the data processing activities. It plays a pivotal role in coordinating the activities of `Executors` and `Workers`, improving of tasks in the generation of Merkle sum tree.
Key functions of the `Orchestrator` include:

- **Dynamic Executor Spawning**: The Orchestrator dynamically spawns `Executors` in numbers set by the user. Each `Executor` is then connected to a dedicated `Worker` for efficient task execution.

- **Task Management and Distribution**: It oversees the overall task flow, loading tasks and distributing them to `Executors`.

- **Error Management and Pipeline Control**: The Orchestrator handles basic pipeline control and responds to errors by initiating the cancellation of all tasks.

## Executor and Worker

The `Executor` acts as a crucial intermediary between the `Orchestrator` and `Workers`, facilitating the data processing workflow. Spawned by the `Orchestrator`, each `Executor` operates in a one-to-one relationship with a `Worker`. The `Worker` is a server that can generate a merkle sum tree, internally called a `mini-tree`, by receiving entries data. The primary role of a `Worker` is to build these `mini-trees`, which are segments of the `AggregationMerkleSumTree`.

Key aspects of the `Executor's` role include:

- **Spawning and Connection**: `Executors` are dynamically spawned by the `Orchestrator` as part of the system's scalability. Each `Executor` is designed to connect with a `Worker` for task execution.

- **Data Handling and Task Distribution**: A primary function of the `Executor` is to receive data entries, often parsed and prepared by the Orchestrator. Upon receiving these entries, the Executor is responsible for forwarding them to its connected `Worker`.

- **Communication Bridge**: The `Executor` serves as a communication bridge within the data pipeline. It relays processed data, `mini-trees`, from Workers back to the Orchestrator.

The `Worker` called in here, mostly point out the container that runs `mini-tree-generator` server.

## ExecutorSpawner

The `ExecutorSpawner` is responsible for initializing and terminating `Executors`. It manages the creation of `Executor` instances and `Workers`, with the latter serving as the `mini-tree-generator` accessible by the `Executor`.

In the Summa-Aggregation, there are three types of `ExecutorSpawner`:

- **MockSpawner**: Primarily used for testing, this spawner initializes `Executors` suitable for various test scenarios, including negative test cases. The Worker spawned by this spawner runs a server locally.

- **LocalSpawner**: It is close to actual use cases, this spawner enables users to initialize `Executors` and `Workers` in local Docker environments.

- **CloudSpawner**: Ideal for scenarios with access to cloud resources, this spawner functions same like to the `LocalSpawner`, but with `Workers` running in the cloud.

While both `LocalSpawner` and `CloudSpawner` manage Docker containers, they differ in operational context. `LocalSpawner` handles individual containers directly, providing simplicity but limited scalability. In contrast, `CloudSpawner` employs Docker Swarm to manage containers as services, thereby offering enhanced scalability and resilience, crucial for larger workloads.

The `ExecutorSpawner` is a trait with minimal requirements, specifically the Rust trait methods `spawn_executor` and `terminate_executor`. You can create your own spawner and use it with the Orchestrator.

## Orchestrating on Swarm

Docker Swarm transforms multiple Docker hosts into a single virtual host, providing crucial capabilities for high availability and scalability in distributed systems like Summa Aggregation.

For more details about Docker Swarm mode, refer to the [official documentation](https://docs.docker.com/engine/swarm/).

### Preparing Docker Swarm Mode

You can initialize your Docker environment in Swarm mode, which is essential for managing a cluster of Docker nodes as a single virtual system.

1. **Activate Swarm Mode on the Main Machine**:
  
    Run the following command to initialize Swarm mode:

    ```bash
    Main $ docker swarm init
    ```

      This command will output information about the Swarm, including a join token.

2. **Join Worker Nodes to the Swarm**:

      Use the join token provided by the main machine to add worker nodes to the swarm. On each worker node, run like:

      ```bash
      Worker_1 $ docker swarm join --token <YOUR_JOIN_TOKEN> <MAIN_MACHINE_IP>:2377
      ```

      Replace `<YOUR_JOIN_TOKEN>` with the actual token and `<MAIN_MACHINE_IP>` with the IP address of your main machine.

3. **Verify Node Status**:
  
      To confirm that the nodes are successfully joined to the swarm, check the node status on the main machine:

      ```bash
      Main $ docker node ls
      ```

      You should see a list of all nodes in the swarm, including their status, roles, and other details like this:

      ```bash
      ID                            HOSTNAME   STATUS    AVAILABILITY   MANAGER STATUS   ENGINE VERSION
      kby50cicvqd5d95o9pgt4puo9 *   main       Ready     Active         Leader           20.10.12
      2adikgxr2l1zp9oqo4kowvw7n     worker_1   Ready     Active                          20.10.12
      dz2z2v7o06h6gazmjlspyr5c8     worker_2   Ready     Active                          20.10.12
      ````

      You are ready to spawn more workers!

### Spawning More Workers

In Docker Swarm mode, containers are managed as services rather than by individual names. To spawn more workers, follow these steps:

1. Deploy the Stack:

    First, deploy your stack using the `docker-compose.yml` file if you haven't already:

    ```bash
    Main $ docker stack deploy -c docker-compose.yml summa_aggregation
    ```

2. Scale the Service:

    Use the docker service scale command to adjust the number of replicas (instances) of your mini-tree service.
    For example, to scale up to 5 instances, run:

    ```bash
    Main $ docker service scale summa_aggregation_mini-tree=5
    ```

    Since each instance has access to all of the worker's resources, it would be appropriate to set the scale number based on the number of workers.

3. Verify the Scaling:

    Check that the service has been scaled properly with:

    ```bash
    Main $ docker service ls
    ```

    This command shows the number of replicas running for each service in the swarm.

Scaling allows you to adjust the number of service instances to meet your processing needs, enhancing the system's capability to handle increased loads or to improve redundancy and availability.

This section provides clear instructions on how to scale the `mini-tree-generator` service using Docker Compose and CLI commands.

## Test

Before starting the tests, you need to build the `mini-tree-generator` image and name it "summa-aggregation".

Build the image with the following command:

```bash
docker build . -t summa-aggregation
```

Ensure that the `summa-aggregation:latest` image exists in your local registry.

Then, you can run the tests using this command:

```bash
cargo test
```

### Test Mini Tree Generator

You can manually test the `Mini Tree Generator` with running container.

First, Use the command below to start the Mini Tree Generator container:

  ```bash
  docker run -d -p 4000:4000 --name mini-tree-generator summa-aggretaion/mini-tree
  ```

Second, to send two entries to the `Mini Tree Generator`, use this script:

  ```bash
  bash ./scripts/test_sending_entry.sh
  ```

Upon successful execution, you will receive a response similar to the following
<details>
<summary>Click View Response</summary>

```Json
{
  "root": {
    "hash": "0x2a4a7ae82b45b3800bdcd6364409e7ba9cac3d4598c546bd48952c234b5d2fb9",
    "balances": [
      "0x000000000000000000000000000000000000000000000000000000000001375f",
      "0x000000000000000000000000000000000000000000000000000000000000e9a6"
    ]
  },
  "nodes": [
    [
      {
        "hash": "0x0e113acd03b98f0bab0ef6f577245d5d008cbcc19ef2dab3608aa4f37f72a407",
        "balances": [
          "0x0000000000000000000000000000000000000000000000000000000000002e70",
          "0x000000000000000000000000000000000000000000000000000000000000a0cb"
        ]
      },
      {
        "hash": "0x17ef9d8ee0e2c8470814651413b71009a607a020214f749687384a7b7a7eb67a",
        "balances": [
          "0x00000000000000000000000000000000000000000000000000000000000108ef",
          "0x00000000000000000000000000000000000000000000000000000000000048db"
        ]
      }
    ],
    [
      {
        "hash": "0x2a4a7ae82b45b3800bdcd6364409e7ba9cac3d4598c546bd48952c234b5d2fb9",
        "balances": [
          "0x000000000000000000000000000000000000000000000000000000000001375f",
          "0x000000000000000000000000000000000000000000000000000000000000e9a6"
        ]
      }
    ]
  ],
  "depth": 1,
  "is_sorted": false
}
```

this JSON output is prettified for clarity

</details>
