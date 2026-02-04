for ((j=0; j<1000; j+=1)); do
  echo "$j $(shuf -n 1 ./examples/lorem-ipsum.txt)"
  sleep 0.3;
done
