package rs.swir.performance.kafkasink;

import org.apache.kafka.clients.producer.ProducerRecord;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.kafka.annotation.KafkaListener;
import org.springframework.kafka.core.KafkaTemplate;
import org.springframework.stereotype.Service;

import java.io.IOException;

@Service
public class KafkaSink {
    private final Logger logger = LoggerFactory.getLogger(rs.swir.performance.kafkasink.KafkaSink.class);

    @Autowired
    KafkaTemplate kafkaTemplate;

    @Value("${sink_producer_topic}")
    String producerTopic;

    @KafkaListener(topics = {"${sink_consumer_topic}"})
    public void consume(String message) throws IOException {
        logger.info(String.format("#### -> Consumed message -> %s", message));
        var f = kafkaTemplate.send(producerTopic, message);

    }
}
