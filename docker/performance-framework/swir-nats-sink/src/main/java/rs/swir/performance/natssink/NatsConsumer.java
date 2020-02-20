package rs.swir.performance.natssink;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import org.springframework.messaging.handler.annotation.SendTo;
import org.springframework.cloud.stream.annotation.EnableBinding;
import org.springframework.cloud.stream.annotation.StreamListener;
import org.springframework.cloud.stream.messaging.Processor;

@EnableBinding(Processor.class)
public class NatsConsumer {
    private final Logger logger = LoggerFactory.getLogger(rs.swir.performance.natssink.NatsConsumer.class);

    @StreamListener(SinkAutoConfiguration.Topics1.INPUT)
    @SendTo(SinkAutoConfiguration.Topics1.OUTPUT)
    public Object consumeT1(Object message) {
        logger.info(String.format("#### -> Consumed message -> %s", message));
        return message;
    }

    @StreamListener(SinkAutoConfiguration.Topics2.INPUT)
    @SendTo(SinkAutoConfiguration.Topics2.OUTPUT)
    public Object consumeT2(Object message) {
        logger.info(String.format("#### -> Consumed message -> %s", message));
        return message;
    }

}
