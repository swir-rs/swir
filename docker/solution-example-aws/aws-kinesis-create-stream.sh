aws kinesis create-stream --stream-name $1 --shard-count=$2
aws kinesis describe-stream --stream-name $1

