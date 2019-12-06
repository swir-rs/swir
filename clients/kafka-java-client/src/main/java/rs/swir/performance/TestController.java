package io.swir.performance;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import io.swir.performance.payload.Payload;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.http.HttpEntity;
import org.springframework.http.ResponseEntity;
import org.springframework.kafka.core.KafkaTemplate;
import org.springframework.kafka.support.SendResult;
import org.springframework.util.concurrent.ListenableFuture;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.client.RestTemplate;

import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

@RestController
public class TestController {
    private static Logger logger = LoggerFactory.getLogger(TestController.class);
    @Autowired
    ObjectMapper om;

    private static final String TOPIC = "Request";

    @Autowired
    private KafkaTemplate<String, String> kafkaTemplate;

    @Autowired
    AtomicInteger processedCounter;


    ExecutorService ex = Executors.newCachedThreadPool();

    @PostMapping("/test")
    public JsonNode handleSwirIncomingStream(@RequestBody() JsonNode body) throws InterruptedException {
        logger.info("Test {}", body);
        int messages = body.get("messages").intValue();
        int threads = body.get("threads").intValue();
        String url =        body.get("sidecarUrl").textValue();
        processedCounter.set(0);
        ObjectMapper om = new ObjectMapper();
        int offset = messages/threads;
        logger.info("url {}",url);
        logger.info("offset {}",offset);
        final AtomicLong totalSendTime  = new AtomicLong(0);

        if(messages % threads !=0){
            ObjectNode response = om.createObjectNode();
            response.put("error", " messges doesn't divide by threads ");
            return response;
        }

        Semaphore semaphore  =new Semaphore(threads);
        final AtomicInteger completedCount = new AtomicInteger();
        long totalStart = System.nanoTime();

        semaphore.acquire(threads);
        for(int i = 0; i <threads; i++) {
            final int k = i;
            ex.submit(new Runnable() {
                public void run() {
                    logger.info("Executing run  {}",k);
                    long start = System.nanoTime();
                    try {
                        for (int j = 0; j < offset; j++) {
                            Payload p = new Payload().setName("kafka-native").setSurname("fooo").setCounter(k * offset + j);
                            ListenableFuture<SendResult<String, String>> f = kafkaTemplate.send(TOPIC, om.writeValueAsString(p));
                            SendResult<String, String> result = f.get();
                            completedCount.incrementAndGet();
                        }
                    }catch(Exception e){
                        logger.error("not right",e);
                    }finally {
                        long stop = System.nanoTime();
                        totalSendTime.addAndGet((stop-start));
                        logger.info("Run {} completed in {}",(stop-start));
                        semaphore.release();
                    }

                }
            });
        };
        semaphore.acquire(threads);

        while(processedCounter.get()!=(messages)){
            logger.debug("completed count {}",processedCounter.get());
            Thread.sleep(10);
        }

        long ts = totalSendTime.get();
        long totalEnd = System.nanoTime();
        long tt = totalEnd-totalStart;
        ObjectNode response = om.createObjectNode();
        response.put("totalSendTimeNs",ts);
        response.put("totalSendTimeMs",TimeUnit.MILLISECONDS.convert(ts,TimeUnit.NANOSECONDS));
        response.put("totalSendTimeS",TimeUnit.SECONDS.convert(ts,TimeUnit.NANOSECONDS));
        response.put("totalTimeNs",tt);
        response.put("totalTimeMs",TimeUnit.MILLISECONDS.convert(tt,TimeUnit.NANOSECONDS));
        response.put("totalTimeS",TimeUnit.SECONDS.convert(tt,TimeUnit.NANOSECONDS));
        response.put("throughput msg/sec",((double)messages/tt)*TimeUnit.NANOSECONDS.convert(1, TimeUnit.SECONDS));
        logger.info("{}",response);
        return response;
    }
}


