while :
do 
  # date +"%D %T.%N";
  len=$((30 + $RANDOM % 50))  
  openssl rand -hex $len
  sleep 0.2;
  # sleep 1;
done
