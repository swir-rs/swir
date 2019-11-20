package io.swir.client;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import io.swir.client.payload.Payload;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.http.HttpEntity;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.client.RestTemplate;

import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicLong;

@RestController
public class TestController {
    private static Logger logger = LoggerFactory.getLogger(IncomingMessagesController.class);
    @Autowired
    ObjectMapper om;

    @Autowired
    RestTemplate restTemplate;

    ExecutorService ex = Executors.newCachedThreadPool();

    @PostMapping("/test")
    public JsonNode handleSwirIncomingStream(@RequestBody() JsonNode body) throws InterruptedException {
        logger.info("Test {}", body);
        int messages = body.get("messages").intValue();
        int threads = body.get("threads").intValue();
        String url =        body.get("sidecarUrl").textValue();

        ObjectMapper om = new ObjectMapper();
        int offset = messages/threads;
        logger.info("url {}",url);
        logger.info("offset {}",offset);
        final AtomicLong totalTime  = new AtomicLong(0);
        Semaphore semaphore  =new Semaphore(threads);

        semaphore.acquire(threads);
        for(int i = 0; i <threads; i++) {
            final int k = i;
            ex.submit(new Runnable() {
                public void run() {
                    logger.info("Executing run  {}",k);
                    long start = System.nanoTime();
                    try {
                        for (int j = 0; j < offset; j++) {
                            HttpEntity<Payload> request = new HttpEntity<>(new Payload().setName("bar").setSurname("fooo").setCounter(k * offset + j));
                            ResponseEntity<String> response = restTemplate.postForEntity(url, request, String.class);
                            if (!response.getStatusCode().is2xxSuccessful()) {
                                logger.warn("Got invalid response {}", response.getStatusCode());
                            }
                        }
                    }catch(Exception e){
                        logger.error("not right",e);
                    }finally {
                        long stop = System.nanoTime();
                        totalTime.addAndGet((stop-start));
                        logger.info("Run {} completed in {}",(stop-start));
                        semaphore.release();
                    }

                }
            });
        };
        semaphore.acquire(threads);
        long tt = totalTime.get();
        logger.info("Done  {} ms throughput {} messages/sec", TimeUnit.MILLISECONDS.convert(tt,TimeUnit.NANOSECONDS), messages/TimeUnit.SECONDS.convert(tt,TimeUnit.NANOSECONDS));

        ObjectNode response = om.createObjectNode();
        response.put("totalTimeNs",tt);
        response.put("totalTimeMs",TimeUnit.MILLISECONDS.convert(tt,TimeUnit.NANOSECONDS));
        response.put("totalTimeS",TimeUnit.SECONDS.convert(tt,TimeUnit.NANOSECONDS));
        response.put("throughput msg/sec",((double)messages/tt)*TimeUnit.NANOSECONDS.convert(1, TimeUnit.SECONDS));
        return response;
    }
}


