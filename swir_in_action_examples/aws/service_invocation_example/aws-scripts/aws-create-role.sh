aws iam create-role --cli-input-yaml file://ecs-role.yaml
aws iam attach-role-policy --role-name swirEcsTaskExecutionRole --policy-arn arn:aws:iam::aws:policy/AmazonDynamoDBFullAccess
aws iam attach-role-policy --role-name swirEcsTaskExecutionRole --policy-arn arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy


