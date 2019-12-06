package io.swir.client;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import io.swir.client.payload.Payload;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.client.RestTemplate;
import org.springframework.web.reactive.function.BodyInserters;
import org.springframework.web.reactive.function.client.WebClient;
import reactor.core.Disposable;
import reactor.core.publisher.Mono;

import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

@RestController
public class TestController {
    private static Logger logger = LoggerFactory.getLogger(TestController.class);
    @Autowired
    ObjectMapper om;

    @Autowired
    RestTemplate restTemplate;

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
        if(messages % threads !=0){
            ObjectNode response = om.createObjectNode();
            response.put("error", " messges doesn't divide by threads ");
            return response;
        }
        int offset = messages/threads;

        logger.info("url {}",url);
        logger.info("offset {}",offset);
        final AtomicLong totalSendTime  = new AtomicLong(0);
        Semaphore semaphore  =new Semaphore(threads);

        WebClient client = WebClient.create(url);

        final AtomicInteger completedCount = new AtomicInteger();
        semaphore.acquire(threads);
        long totalStart = System.nanoTime();
        for(int i = 0; i <threads; i++) {
            final int k = i;
            ex.submit(new Runnable() {
                public void run() {
                    logger.info("Executing run  {}",k);
                    long start = System.nanoTime();
                    try {
                        for (int j = 0; j < offset; j++) {
                            final int c = k * offset + j;
                            final WebClient.RequestHeadersSpec<?> request = client.post().uri("/publish").body(BodyInserters.fromPublisher(Mono.just(new Payload().setName("client").setSurname("fooo").setCounter(c)), Payload.class));
                            Disposable resp = request.retrieve().bodyToMono(String.class).map(f->{logger.info("Got response {} for {}",f,c);completedCount.incrementAndGet();return f;}).subscribe();
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


