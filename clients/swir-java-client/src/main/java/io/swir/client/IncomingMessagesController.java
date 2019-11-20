package io.swir.client;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;

@RestController
public class IncomingMessagesController {
    private static Logger logger = LoggerFactory.getLogger(IncomingMessagesController.class);
    @Autowired
    ObjectMapper om;

    @PostMapping("/response")
    public JsonNode handleSwirIncomingStream(@RequestBody() JsonNode body) {
        logger.info("Incoming message {}", body);
        return null;
    }
}
