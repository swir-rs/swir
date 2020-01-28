package rs.swir.performance.natssink;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import org.springframework.messaging.handler.annotation.SendTo;
import org.springframework.cloud.stream.annotation.EnableBinding;
import org.springframework.cloud.stream.annotation.StreamListener;
import org.springframework.cloud.stream.messaging.Processor;
import org.springframework.messaging.handler.annotation.SendTo;

@EnableBinding(Processor.class)
public class NatsConsumer {
    private final Logger logger = LoggerFactory.getLogger(rs.swir.performance.natssink.NatsConsumer.class);



    @StreamListener(Processor.INPUT)
    @SendTo(Processor.OUTPUT)
    public Object consume(Object message) {
        logger.info(String.format("#### -> Consumed message -> %s", message));
        return message;
    }

}
