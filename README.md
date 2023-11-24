# Summa Aggregation

Summa Aggregation is focused on optimizing the generation of Merkle sum trees, a task identified as the primary time-consuming process in Summa benchmarks. Our goal is to significantly reduce the time required to generate these trees by leveraging parallelization across multiple machines.

The system features `AggregationMerkleSumTree`, a specialized component designed for the efficient construction of complete trees from smaller, aggregated structures known as `mini-trees`. This approach not only speeds up the tree-building process but also enhances scalability and performance in large-scale data environments.

## Orchestrator

The Orchestrator component will manage the orchestration of multiple `Executors`. Its primary role will be to dynamically allocate tasks and manage the data workflow among the `Executors`.

## Executor and ExecutorSpawner

The `Executor` plays a crucial role in the data processing pipeline. It is responsible for receiving parsed data entries and processing them through the `mini-tree-generator`.
Each `Executor` operates alongside a `mini-tree-generator`, accessed via a URL spawned in a container by the `ExecutorSpawner`.

The `ExecutorSpawner` is a trait that initializes and manages these `Executors`. It handles the creation of `Executor` instances and manages the worker, `mini-tree-generator`, which is linked with the `Executor`.

### Executor Workflow

- `Executors` receive data entries, parsed from CSV files.
- They send these entries to the `mini-tree-generator` service.
- The `mini-tree-generator` processes these entries and returns the results as tree structures.
- `Executors` then collect and forward these results for further aggregation or processing.

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

      You are ready to scaling!

### Scaling Worker

To scale the service, follow these steps:

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

### ServiceSpawner and Orchestrator

In Summa Aggregation, we have implemented two types of spawners for the `Orchestrator`: the `container spawner` and the `service spawner`. These spawners are instrumental in managing the deployment and operation of our processing units, whether they are individual containers or services within a Docker Swarm environment.

- **Container Spawner**:
  The `container spawner` operates in local Docker environments. It is primarily used for development and testing purposes, This spawner is ideal for situations where simplicity and ease of setup are key, such as during initial development phases or for running unit tests.

- **Service Spawner**:
  The `service spawner` is designed to work with Docker Swarm environments. It is suitable for production deployments, where the system needs to scale across multiple machines or nodes. This spawner leverages Docker Swarm's orchestration capabilities to manage services, ensuring that they are reliably deployed, scaled, and maintained across the swarm.

While both spawners manage Docker containers, the key difference lies in their operational context. The `container spawner` handles individual containers directly, making it straightforward but less scalable. On the other hand, the `service spawner` interacts with Docker Swarm to manage groups of containers as services, offering more robust scalability and resilience, crucial for handling larger workloads or distributed systems.

## Test

Before start to test, you have to build `mini-tree-generator` image as name of "summa-aggregation".

You can build image with this command:

```bash
$ docker build . -t summa-aggregation
```

## Mini Tree Generator

- Build the Image
  
  To build the image, run the following command:

  ```bash
  docker build . -t summa-aggregation/mini-tree
  ```

- Run the `Mini Tree Generator Container`

  Use the command below to start the Mini Tree Generator container:

  ```bash
  docker run -d -p 4000:4000 --name mini-tree-generator summa-aggretaion/mini-tree
  ```

- Test with a Script

  To test, execute the provided script that send two `Entry` data to server:

  ```bash
  bash ./scripts/test_sending_entry.sh
  ```

  Upon successful execution, you will receive a response similar to the following
  <details>
  <summary>Response Json!</summary>

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
    "entries": [
      {
        "balances": [
          "11888",
          "41163"
        ],
        "username": "dxGaEAii"
      },
      {
        "balances": [
          "67823",
          "18651"
        ],
        "username": "MBlfbBGI"
      }
    ],
    "is_sorted": false
  }
  ```

  this JSON output is prettified for clarity

</details>
