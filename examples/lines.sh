for ((j=0; j<1000; j+=1)); do
  result=''
  n=$((1 + $RANDOM % 4))
  for ((i=0; i<n; i+=1)); do
    len=$((2 + $RANDOM % 32))  
    s=$(openssl rand -hex $len)
    result="$result $s"
  done

  echo "$j $result"
  sleep 0.1;
  # sleep 1;
done
