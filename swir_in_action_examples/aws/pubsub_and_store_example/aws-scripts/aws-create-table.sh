aws dynamodb create-table --table-name $1 --key-schema AttributeName=partition_key,KeyType=HASH --attribute-definitions AttributeName=partition_key,AttributeType=S --billing-mode=PROVISIONED --provisioned-throughput=ReadCapacityUnits=5,WriteCapacityUnits=5