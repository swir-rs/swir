### In this exmaple SWIR sidecars are used to :

 * Store data in DynamoDB
 * Send and receive messages to/from Amazon Kinesis

### PubSub and Store Example in Aws
![PubSub and Store Example in Kubernetes](../../../graphics/example-aws-solution.png)

### Running this example:

```./run_example.sh amazonID region```

you need to export your AWS credentials as environment variables in ../../../secure.sh as shown below

```
export AWS_ACCESS_KEY=Swir  
export AWS_SECURE_ACCESS_KEY=Swir
```

### Cleaning up resources:

```./cleanup_example.sh clusterID``` 

where clusterID is printed when the ECS cluster is created
