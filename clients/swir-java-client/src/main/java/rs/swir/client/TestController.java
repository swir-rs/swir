package rs.swir.client;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
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

import javax.validation.constraints.NotNull;

import java.time.Duration;
import java.util.Map;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

@RestController
public class TestController {
    private static Logger logger = LoggerFactory.getLogger(TestController.class);
    private final CBORFactory f;

    @NotNull
    @Value("${sidecarUrl}")
    private String sidecarUrl;

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
    public JsonNode handleSwirIncomingStream(@RequestBody() JsonNode body) throws InterruptedException {
        logger.info("Test {}", body);
        var messages = body.get("messages").intValue();
        var threads = body.get("threads").intValue();
        var clientTopic = body.get("clientTopic").textValue();
        String url = body.get("sidecarUrl").textValue();
        var missedPacketsThreshold = 1;
        if(body.hasNonNull("missedPackets")) {
            missedPacketsThreshold = body.get("missedPackets").intValue();
        }
        String testType;
        if(body.hasNonNull("testType")) {
            testType = body.get("testType").textValue();
        }else{
            testType = "sidecar";
        }
        processedCounter.set(0);
        ObjectMapper om = new ObjectMapper();
        ObjectMapper omCbor = new ObjectMapper(f);
        if(messages % threads !=0){
            ObjectNode response = om.createObjectNode();
            response.put("error", " messages doesn't divide by threads ");
            return response;
        }
        int offset = messages/threads;

        logger.info("url {}",url);
        logger.info("offset {}",offset);
        testStarted.set(true);
        final AtomicLong totalSendTime  = new AtomicLong(0);
        Semaphore semaphore  =new Semaphore(threads);

        final Map<String, String> headersMap = Map.of("content-type","application/octet-stream","topic",clientTopic);

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
                            switch(testType){
                                case "sidecar":
                                    sendMessageViaSidecarFlux(c, om, clientTopic, sentCount);break;
                                case "kafka":
                                    sendMessageToKafkaDirectly(c,om,kafkaProducerTopic,sentCount);break;
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

        while(sentCount.get()!=messages){
            Thread.sleep(100);
        }
        logger.info("sent count {} {}",sentCount.get(),messages);

        while(processedCounter.get()!=(messages) && !missedPackets){
            int count = processedCounter.get();
            logger.info("completed count {} {}",count,oldCount);
            if(oldCount==count){
                missingPacketCounter++;
                logger.warn("Count has not changed {}",missingPacketCounter );
            }
            if(missingPacketCounter> missedPacketsThreshold){
                missedPackets =true;
                logger.error("Count has not changed {}",missingPacketCounter );
            }
            oldCount = count;
            Thread.sleep(100);
        }
        long ts = totalSendTime.get();
        long totalEnd = System.nanoTime();
        long tt = totalEnd-totalStart;
        testStarted.set(false);
        ObjectNode response = om.createObjectNode();
        response.put("totalSendTimeNs",ts);
        response.put("totalSendTimeMs",TimeUnit.MILLISECONDS.convert(ts,TimeUnit.NANOSECONDS));
        response.put("totalSendTimeS",TimeUnit.SECONDS.convert(ts,TimeUnit.NANOSECONDS));
        response.put("totalTimeNs",tt);
        response.put("totalTimeMs",TimeUnit.MILLISECONDS.convert(tt,TimeUnit.NANOSECONDS));
        response.put("totalTimeS",TimeUnit.SECONDS.convert(tt,TimeUnit.NANOSECONDS));
        response.put("throughput msg/sec",((double)messages/tt)*TimeUnit.NANOSECONDS.convert(1, TimeUnit.SECONDS));
        response.put("packets missed ",messages- processedCounter.get());
        logger.info("{}",response);
        return response;
    }

    void sendMessageViaSidecarFlux(int c, ObjectMapper om, String clientTopic, AtomicInteger sentCount  ) throws JsonProcessingException {
        var p = new Payload().setName("client").setSurname("fooo").setCounter(c);
        logger.info("sending request {}",p);
        final Map<String, String> headersMap = Map.of("content-type","application/octet-stream","topic",clientTopic);
        final WebClient.RequestHeadersSpec<?> request = client.post().uri(sidecarUrl)
                .headers(httpHeaders -> httpHeaders.setAll(headersMap))
                .body(BodyInserters.fromValue(om.writeValueAsBytes(p)));

        var resp = request.exchange().delaySubscription(Duration.ofMillis(2)).subscribe();
        sentCount.incrementAndGet();
    }

    void sendMessageViaSidecarClassic(int c, ObjectMapper om, String clientTopic, AtomicInteger sentCount  ) throws JsonProcessingException {
        var p = new Payload().setName("client").setSurname("fooo").setCounter(c);
        logger.info("sending request {}",p);
        HttpHeaders headers = new HttpHeaders();
        headers.add("content-type", "application/octet-stream");
        headers.add("topic", clientTopic);
        var request = new HttpEntity<>(om.writeValueAsString(p), headers);
        restTemplate.postForEntity(sidecarUrl,request,String.class);
        sentCount.incrementAndGet();

    }


    void sendMessageToKafkaDirectly(int c, ObjectMapper om, String clientTopic, AtomicInteger sentCount  ) throws JsonProcessingException, ExecutionException, InterruptedException {
        Payload p = new Payload().setName("kafka-native").setSurname("fooo").setCounter(c);
        logger.info("sending request {}",p);
        var f = kafkaTemplate.send(clientTopic, om.writeValueAsString(p));
        f.get();
        sentCount.incrementAndGet();

    }
}


