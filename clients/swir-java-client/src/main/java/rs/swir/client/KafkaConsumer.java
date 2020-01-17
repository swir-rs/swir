package rs.swir.client;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.kafka.annotation.KafkaListener;
import org.springframework.stereotype.Service;

import java.io.IOException;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;

@Service
public class KafkaConsumer {
    private final Logger logger = LoggerFactory.getLogger(KafkaConsumer.class);

    @Autowired
    AtomicBoolean testStarted;

    @Autowired
    AtomicInteger processedCounter;

    @KafkaListener(topics="${kafkaConsumerTopic}")
    public void consume(String message) throws IOException {
        if(testStarted.get()) {
            processedCounter.incrementAndGet();
            logger.info(String.format("#### -> Incoming message  -> %s", message));
        }else{
            logger.warn(String.format("#### -> Incoming message  -> %s", message));
        }
    }
}

