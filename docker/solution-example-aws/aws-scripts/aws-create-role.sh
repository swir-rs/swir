aws iam create-role --cli-input-yaml file://ecs-role.yaml
aws iam attach-role-policy --role-name swirEcsTaskExecutionRole --policy-arn arn:aws:iam::aws:policy/AmazonDynamoDBFullAccess
aws iam attach-role-policy --role-name swirEcsTaskExecutionRole --policy-arn arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy
aws iam put-role-policy --role-name swirEcsTaskExecutionRole --policy-name swir-specific --policy-document '{"Version": "2012-10-17","Statement": [{"Sid": "VisualEditor0","Effect": "Allow","Action": ["kinesis:PutRecord","kinesis:DescribeStreamSummary","dynamodb:PutItem",                "kinesis:ListShards","kinesis:PutRecords","kinesis:GetShardIterator","dynamodb:GetItem","kinesis:GetRecords","kinesis:DescribeStream","dynamodb:UpdateItem"],"Resource": "*"}]}'

