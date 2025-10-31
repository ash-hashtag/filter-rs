for ((j=0; j<1000; j+=1)); do
  result=''
  echo "$(shuf -n 1 ~/git/libsignal/README.md)"
  sleep 0.5;
done
# for ((j=0; j<1000; j+=1)); do
#   result=''
#   n=$((1 + $RANDOM % 8))
#   for ((i=0; i<n; i+=1)); do
#     len=$((2 + $RANDOM % 32))  
#     s=$(openssl rand -hex $len)
#     result="$s $result"
#   done

#   echo "$result"
#   sleep 0.5;
# done
