# Summa Aggregation

For demonstration, We only run `Executor` instead of `Orchestrator` what we planned to implemented in here.`

## Orchestrator

The orchestrator is not yet implemented. In a real-world scenario, the orchestrator will spawn multiple `Executors`.

The Executors' roles include loading `csv` files and sending data, which is converted from csv file as json type, to the Worker, which runs the `MiniTreeGenerator` in a Docker container.

## Demo

The demonstration compares the performance of processing large entries with a single machine (container) against processing smaller, chunked entries with four machines (containers). To run multiple `Workers`, use:

```
$ docker-compose up -d --scale mini-tree=4
```

Note That, Each worker container is assigned 1 CPU core and 128MB RAM for comparison.

And you can see the comparison result with this command:

```
cargo run --bin summa-aggregation
```

## Mini Tree Generator

- Build the Image
  
  To build the image, run the following command:
  ```
  docker build . -t summa-aggregation/mini-tree
  ```

- Run the `Mini Tree Generator Container`

  Use the command below to start the Mini Tree Generator container:

  ```
  docker run -d -p 4000:4000 --name mini-tree-generator summa-aggretaion/mini-tree
  ```

- Test with a Script

  To test, execute the provided script that send two `Entry` data to server:
  ```
  bash ./scripts/test_sending_entry.sh
  ```

  Upon successful execution, you will receive a response similar to the following 
  (JSON output is prettified for clarity):
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
