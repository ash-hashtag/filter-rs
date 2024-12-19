while :
do 
  # date +"%D %T.%N";
  result=''
  n=$((1 + $RANDOM % 4))
  for ((i=0; i<n; i+=1)); do
    len=$((2 + $RANDOM % 32))  
    s=$(openssl rand -hex $len)
    result="$result $s"
  done

  echo $result
  sleep 0.2;
  # sleep 1;
done
