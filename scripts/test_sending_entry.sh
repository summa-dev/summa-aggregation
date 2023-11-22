#!/bin/bash

# Check if a port number is provided
if [ $# -eq 0 ]
then
    echo "No port number supplied. Using default port 4000."
    PORT=4000
else
    PORT=$1
fi

# The curl command with the variable PORT
curl -X POST http://localhost:$PORT/ \
     -vvv \
     -H "Content-Type: application/json" \
     -d '[
            {
                "balances": ["11888", "41163"],
                "username": "dxGaEAii"
            },
            {
                "balances": ["67823", "18651"],
                "username": "MBlfbBGI"
            },
             {
                "balances": ["11888", "41163"],
                "username": "dxGaEAii"
            },
            {
                "balances": ["67823", "18651"],
                "username": "MBlfbBGI"
            },
            {
                "balances": ["11888", "41163"],
                "username": "dxGaEAii"
            },
            {
                "balances": ["67823", "18651"],
                "username": "MBlfbBGI"
            },
             {
                "balances": ["11888", "41163"],
                "username": "dxGaEAii"
            },
            {
                "balances": ["67823", "18651"],
                "username": "MBlfbBGI"
            }
         ]'
