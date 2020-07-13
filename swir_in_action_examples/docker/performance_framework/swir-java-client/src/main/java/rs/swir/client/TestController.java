package rs.swir.client;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.fasterxml.jackson.dataformat.cbor.CBORFactory;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.http.HttpEntity;
import org.springframework.http.HttpHeaders;
import org.springframework.kafka.core.KafkaTemplate;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.client.RestTemplate;
import org.springframework.web.reactive.function.BodyInserters;
import org.springframework.web.reactive.function.client.WebClient;
import rs.swir.client.payload.Payload;

import java.net.InetAddress;
import java.net.UnknownHostException;
import java.time.Duration;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

@RestController
public class TestController {
    private static Logger logger = LoggerFactory.getLogger(TestController.class);
    private final CBORFactory f;


    private static final Random random = new Random();
    private static final String SIDECAR_TEST_TYPE= "sidecar";

//    @NotNull
//    @Value("${sidecarUrl}")
//    private String sidecarUrl;

    @Autowired
    RestTemplate restTemplate;

    @Autowired
    AtomicInteger processedCounter;

    @Autowired
    AtomicBoolean testStarted;

    @Autowired
    private KafkaTemplate<String, String> kafkaTemplate;

    ExecutorService ex = Executors.newCachedThreadPool();
    @Value("${kafkaProducerTopic}")
    String kafkaProducerTopic;

    @Autowired
    WebClient client;


    TestController(){
        f = new CBORFactory();
    }

    @PostMapping("/test")
    public JsonNode handleSwirIncomingStream(@RequestBody() JsonNode body) throws InterruptedException, UnknownHostException, JsonProcessingException {
        logger.info("Test {}", body);
        var messages = body.get("messages").intValue();
        var threads = body.get("threads").intValue();

        var producerTopics = (ArrayNode)body.get("producerTopics");
        var subscriberTopics = (ArrayNode)body.get("subscriberTopics");

        String sidecarUrl = body.get("sidecarUrl").textValue();

        var missedPacketsThreshold = 1;
        if(body.hasNonNull("missedPackets")) {
            missedPacketsThreshold = body.get("missedPackets").intValue();
        }

        var clientUrl = "http://"+InetAddress.getLocalHost().getHostAddress()+":8090/response";
        var clientId = UUID.randomUUID().toString();
        String testType;
        if(body.hasNonNull("testType")) {
            testType = body.get("testType").textValue();
        }else{
            testType = "sidecar";
        }
        processedCounter.set(0);
        ObjectMapper om = new ObjectMapper();
        if(messages % threads !=0){
            ObjectNode response = om.createObjectNode();
            response.put("error", " messages doesn't divide by threads ");
            return response;
        }
        int offset = messages/threads;

        logger.info("url {}",sidecarUrl);
        logger.info("offset {}",offset);

        var correlationIds=  new ArrayList<String>();


        if(testType==SIDECAR_TEST_TYPE) {
            var iter = subscriberTopics.elements();
            while (iter.hasNext()){
                String correlationId = UUID.randomUUID().toString();
                correlationIds.add(correlationId);
                var subscriberTopic = iter.next().asText();
                subscribeForMessagesFromSidecar(sidecarUrl,om,subscriberTopic, clientUrl+"-"+subscriberTopic, clientId,correlationId);
            }
        }

        Thread.sleep(5000);

        testStarted.set(true);
        final AtomicLong totalSendTime  = new AtomicLong(0);
        Semaphore semaphore  =new Semaphore(threads);

        final AtomicInteger sentCount = new AtomicInteger();
        semaphore.acquire(threads);
        long totalStart = System.nanoTime();
        for(int i = 0; i <threads; i++) {
            final int k = i;
            ex.submit(new Runnable() {
                public void run() {
                    logger.info("Executing run {} {}",testType, k);
                    long start = System.nanoTime();
                    try {
                        for (int j = 0; j < offset; j++) {
                            final int c = k * offset + j;

                            int t =0;
                            var iter =producerTopics.elements();
                            while(iter.hasNext()) {
                                var producerTopic = iter.next().asText();
                                switch (testType) {
                                    case SIDECAR_TEST_TYPE:
                                        sendMessageViaSidecarFlux(sidecarUrl, c, om, producerTopic, sentCount,producerTopic,subscriberTopics.get(t++).asText());
                                        break;
                                    case "kafka":
                                        sendMessageToKafkaDirectly(c, om, kafkaProducerTopic, sentCount,producerTopic,subscriberTopics.get(t++).asText());
                                        break;
                                }
                            }
                        }
                    }catch(Exception e){
                        logger.error("not right",e);
                    }finally {
                        long stop = System.nanoTime();
                        totalSendTime.addAndGet((stop-start));
                        logger.info("Run {} completed in {}",k,(stop-start));
                        semaphore.release();
                    }
                }
            });
        };
        semaphore.acquire(threads);
        int oldCount =0;
        int missingPacketCounter = 0;
        boolean missedPackets = false;
        var totalMessages = messages*producerTopics.size();
        while(sentCount.get()!=totalMessages){
            Thread.sleep(100);
        }
        logger.info("sent count {} {}",sentCount.get(),totalMessages);

        while(processedCounter.get()!=(totalMessages) && !missedPackets){
            int count = processedCounter.get();
            logger.info("completed count {} {}",count,oldCount);
            if(oldCount==count){
                missingPacketCounter++;
                logger.warn("Count has not changed {}",missingPacketCounter );
            }else{
		        missingPacketCounter=0;
	        }
            if(missingPacketCounter> missedPacketsThreshold){
                missedPackets =true;
                logger.error("Count has not changed {}",missingPacketCounter );
            }
            oldCount = count;
            Thread.sleep(100);
        }
        long ts = totalSendTime.get()/threads;
        long totalEnd = System.nanoTime();
        long tt = totalEnd-totalStart;
        testStarted.set(false);

        int i= 0;
        if(testType==SIDECAR_TEST_TYPE) {
            var iter =subscriberTopics.elements();
            while (iter.hasNext()){
                var correlationId = correlationIds.get(i++);
                var subscriberTopic = iter.next().asText();
                unsubscribeForMessagesFromSidecar(sidecarUrl, om, subscriberTopic, clientUrl+"-"+subscriberTopic, clientId,correlationId);
            }
        }

        ObjectNode response = om.createObjectNode();

        response.put("totalSendTimeNs",ts);
        response.put("totalSendTimeMs",TimeUnit.MILLISECONDS.convert(ts,TimeUnit.NANOSECONDS));
        response.put("totalSendTimeS",TimeUnit.SECONDS.convert(ts,TimeUnit.NANOSECONDS));
        response.put("totalTimeNs",tt);
        response.put("totalTimeMs",TimeUnit.MILLISECONDS.convert(tt,TimeUnit.NANOSECONDS));
        response.put("totalTimeS",TimeUnit.SECONDS.convert(tt,TimeUnit.NANOSECONDS));
        response.put("throughput msg/sec",((double)totalMessages/tt)*TimeUnit.NANOSECONDS.convert(1, TimeUnit.SECONDS));
        response.put("packets missed ",totalMessages- processedCounter.get());
        logger.info("{}",response);
        return response;
    }


    void sendMessageViaSidecarFlux(String sidecarUrl, int c, ObjectMapper om, String clientTopic, AtomicInteger sentCount, String clientName, String consumerName) throws JsonProcessingException {
        byte[] bytes = new byte[16];
        random.nextBytes(bytes);;
        var p = new Payload().setProducer(clientName).setConsumer(consumerName).setCounter(c).setTimestamp(System.currentTimeMillis()).setPayload(Base64.getEncoder().encodeToString(bytes));

        logger.info("sending request {}",p);
        final Map<String, String> headersMap = Map.of("content-type","application/octet-stream","topic",clientTopic,"X-Correlation-ID", UUID.randomUUID().toString());
        final WebClient.RequestHeadersSpec<?> request = client.post().uri(sidecarUrl+"/pubsub/publish")
                .headers(httpHeaders -> httpHeaders.setAll(headersMap))
                .body(BodyInserters.fromValue(om.writeValueAsBytes(p)));

        var resp = request.exchange().delaySubscription(Duration.ofMillis(2)).subscribe();
        sentCount.incrementAndGet();
    }


    void sendMessageViaSidecarClassic(String sidecarUrl, int c, ObjectMapper om, String clientTopic, AtomicInteger sentCount,String client,String consumer  ) throws JsonProcessingException {
        byte[] bytes = new byte[64];
        random.nextBytes(bytes);;
        var p = new Payload().setProducer(client).setConsumer(consumer).setCounter(c).setTimestamp(System.currentTimeMillis()).setPayload(Base64.getEncoder().encodeToString(bytes));

        logger.info("sending request {}",p);
        HttpHeaders headers = new HttpHeaders();
        final Map<String, String> headersMap = Map.of("content-type","application/octet-stream","topic",clientTopic,"X-Correlation-ID", UUID.randomUUID().toString());
        headers.add("topic", clientTopic);
        var request = new HttpEntity<>(om.writeValueAsString(p), headers);
        restTemplate.postForEntity(sidecarUrl+"/pubsub/publish",request,String.class);
        sentCount.incrementAndGet();

    }


    void sendMessageToKafkaDirectly(int c, ObjectMapper om, String clientTopic, AtomicInteger sentCount, String client,String consumer  ) throws JsonProcessingException, ExecutionException, InterruptedException {
        byte[] bytes = new byte[64];
        random.nextBytes(bytes);;
        var p = new Payload().setProducer(client).setConsumer(consumer).setCounter(c).setTimestamp(System.currentTimeMillis()).setPayload(Base64.getEncoder().encodeToString(bytes));
        logger.info("sending request {}",p);
        var f = kafkaTemplate.send(clientTopic, om.writeValueAsString(p));
        f.get();
        sentCount.incrementAndGet();

    }

    public void subscribeForMessagesFromSidecar(String sidecarUrl, ObjectMapper om,String topic, String clientUrl,String clientId,String correlationId) throws JsonProcessingException {
        var p = new ClientSubscribeRequest().setClientTopic(topic).setEndpoint(new EndpointDescription().setUrl(clientUrl).setClientId(clientId));
        logger.info("subscribe request {}",p);
        final Map<String, String> headersMap = Map.of("content-type","application/json","topic",topic,"X-Correlation-ID", correlationId);
        final WebClient.RequestHeadersSpec<?> request = client.post().uri(sidecarUrl+"/pubsub/subscribe")
                .headers(httpHeaders -> httpHeaders.setAll(headersMap))
                .body(BodyInserters.fromValue(om.writeValueAsBytes(p)));

        var resp = request.exchange().block();
        var body = resp.bodyToMono(String.class).block();
        logger.info(String.format("Subscription status = %s , %s", resp.statusCode(),body));
    }

    public void unsubscribeForMessagesFromSidecar(String sidecarUrl,ObjectMapper om,String topic, String clientUrl,String clientId,String correlationId) throws JsonProcessingException {
        var p = new ClientSubscribeRequest().setClientTopic(topic).setEndpoint(new EndpointDescription().setUrl(clientUrl).setClientId(clientId));
        logger.info("unsubscribe request {}",p);
        final Map<String, String> headersMap = Map.of("content-type","application/json","topic",topic,"X-Correlation-ID", correlationId);
        final WebClient.RequestHeadersSpec<?> request = client.post().uri(sidecarUrl+"/pubsub/unsubscribe")
                .headers(httpHeaders -> httpHeaders.setAll(headersMap))
                .body(BodyInserters.fromValue(om.writeValueAsBytes(p)));

        var resp = request.exchange().block();
        var body = resp.bodyToMono(String.class).block();
        logger.info(String.format("Subscription status = %s , %s", resp.statusCode(),body));

    }
}


