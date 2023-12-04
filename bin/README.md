# Mini Tree Server

Mini tree server is wrapped 

### Test Mini Tree Server

You can manually test the `Mini Tree Server` with running container.

First, Use the command below to start the Mini Tree Server container:

  ```bash
  docker run -d -p 4000:4000 --name mini-tree-server summa-aggretaion/mini-tree
  ```

Second, to send two entries to the `Mini Tree Server`, use this script:

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
