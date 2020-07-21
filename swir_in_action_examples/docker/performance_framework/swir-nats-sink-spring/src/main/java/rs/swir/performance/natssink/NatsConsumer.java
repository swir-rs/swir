package rs.swir.performance.natssink;

import com.google.protobuf.InvalidProtocolBufferException;
import nats_msg_wrapper.NatsMsgWrapper;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import org.springframework.messaging.handler.annotation.SendTo;
import org.springframework.cloud.stream.annotation.EnableBinding;
import org.springframework.cloud.stream.annotation.StreamListener;
import org.springframework.cloud.stream.messaging.Processor;
import org.apache.commons.codec.binary.Hex;

@EnableBinding(Processor.class)
public class NatsConsumer {
    private final Logger logger = LoggerFactory.getLogger(rs.swir.performance.natssink.NatsConsumer.class);

    @StreamListener(SinkAutoConfiguration.Topics1.INPUT)
    @SendTo(SinkAutoConfiguration.Topics1.OUTPUT)
    public Object consumeT1(Object message) {
        try {
            String payload = (String)message;
            logger.info(String.format("#### -> Consumed raw message -> %s", Hex.encodeHexString( payload.getBytes() )));
            NatsMsgWrapper.NatsMessageWrapper msg = NatsMsgWrapper.NatsMessageWrapper.parseFrom(payload.getBytes());
            logger.info(String.format("#### -> Consumed message -> %s", msg));
            return message;
        } catch (InvalidProtocolBufferException e) {
            logger.error("consumeT1 " + e.getMessage());
            return message;
        }

    }

    @StreamListener(SinkAutoConfiguration.Topics2.INPUT)
    @SendTo(Processor.OUTPUT)
    public Object consumeT2(Object message) {
        try {
            String payload = (String)message;
            logger.info(String.format("#### -> Consumed raw message -> %s", Hex.encodeHexString( payload.getBytes() )));
            NatsMsgWrapper.NatsMessageWrapper msg = NatsMsgWrapper.NatsMessageWrapper.parseFrom(payload.getBytes());
            logger.info(String.format("#### -> Consumed message -> %s", msg));
            return message;
        } catch (InvalidProtocolBufferException e) {
            logger.error("consumeT2 " + e.getMessage());
            return message;
        }
    }

}
