#!/bin/bash

while true; do
    echo "Printing... Press 'x' to stop."
    sleep 0.1  # Adjust the sleep time to control the print speed
    if read -n 1 -t 0.1 input && [[ $input == "x" ]]; then
        break
    fi
done
