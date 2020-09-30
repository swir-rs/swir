### In this exmaple SWIR sidecars are used to :

 * Invoke REST services exposed by other applications
 * Create a zero config service mesh where services are discoverable through mDNS
 * Protect the integrity of traffic with mTLS
 * Collect tracing logs and display them through Jaeger UI

### Service Discovery and Invocation
![Service Discovery and Invocation](../../../graphics/example-solution-sdi.png)


### Running this example :

```./run_example.sh si-example swir-operator-ns```

#### it will take a while for a solution to stabilize

### Get URL for tracing dashboard

```../tracing_dashboard.sh si-example```

### To check logs run : 

```./display_books_logs.sh si-example ```

```./display_helpdesk_logs.sh si-example ```


### Cleaning up resources:

```./cleanup_example.sh si-example swir-operator-ns```


