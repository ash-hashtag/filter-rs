for ((j=0; j<1000; j+=1)); do
  result=''
  echo "$(shuf -n 1 ./examples/lorem-ipsum.txt)"
  sleep 0.5;
done
