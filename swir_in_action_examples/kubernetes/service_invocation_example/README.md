### In this exmaple SWIR sidecars are used to :

 * Invoke REST services exposed by other applications
 * Create a zero config service mesh where services are discoverable through mDNS
 * Protect the integrity of traffic with mTLS
 * Collect tracing logs and display them through Jaeger UI

### Service Discovery and Invocation
![Service Discovery and Invocation](../../../graphics/example-solution-sdi.png)


### Running this example:

```./cicd.sh```

### Get URL for tracing dashboard

```../tracing_dashboard.sh```


### Cleaning up resources:

```./cicd_cleanup.sh```
