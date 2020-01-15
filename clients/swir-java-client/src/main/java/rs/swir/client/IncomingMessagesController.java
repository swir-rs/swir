package rs.swir.client;


import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.dataformat.cbor.CBORFactory;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;
import reactor.core.publisher.Mono;

import javax.validation.Payload;
import java.util.concurrent.atomic.AtomicInteger;

@RestController
public class IncomingMessagesController {
    private static Logger logger = LoggerFactory.getLogger(IncomingMessagesController.class);
    private final CBORFactory f;
    private final ObjectMapper om;


    IncomingMessagesController(){
        f = new CBORFactory();
        om = new ObjectMapper();
    }


    @Autowired
    AtomicInteger processedCounter;

    @PostMapping("/response")
    public Mono<byte[]> handleSwirIncomingStream(@RequestBody() byte[]  body) {
        Payload p = null;
        try {
            p = om.readValue(body, Payload.class);
            logger.info("Incoming message {}", p.toString());
        } catch (Exception e) {
            logger.error(e.getLocalizedMessage());
        }

        processedCounter.incrementAndGet();
        return null;
    }
}
