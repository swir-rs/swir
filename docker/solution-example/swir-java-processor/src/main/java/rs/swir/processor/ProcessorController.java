package rs.swir.processor;


import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.boot.web.server.LocalServerPort;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestHeader;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.reactive.function.BodyInserters;
import org.springframework.web.reactive.function.client.WebClient;

import javax.annotation.PostConstruct;
import java.net.InetAddress;
import java.net.UnknownHostException;
import java.time.Duration;
import java.util.Map;
import java.util.UUID;

@RestController
public class ProcessorController {
    private static Logger logger = LoggerFactory.getLogger(ProcessorController.class);

    @Autowired
    WebClient client;

    @Value("${producer_topic}")
    String producerTopic;

    @Value("${subscribe_topic}")
    String subscribeTopic;

    @Value("${sidecar_url}")
    String sidecarUrl;

    @Value("${server.port}")
    int port;

    static final String correlationId = UUID.randomUUID().toString();
    static final String clientId = "Processor-"+UUID.randomUUID().toString();

    @PostConstruct
    public void subscribe() throws JsonProcessingException, UnknownHostException {
        var clientUrl = "http://"+ InetAddress.getLocalHost().getHostAddress()+":"+port+"/response";
        subscribeForMessagesFromSidecar(clientUrl, subscribeTopic, clientId, correlationId);
    }


    @PostMapping("/response*")
    public void handleSwirIncomingStream(@RequestBody() byte[]  body) {

        logger.info(String.format("processing : %s %s", new String(body),correlationId));
        sendMessageViaSidecarFlux(body,correlationId);
    }


    void sendMessageViaSidecarFlux(byte[] payload, String correlationId) {
        logger.debug("sending request {}", new String(payload));
        final Map<String, String> headersMap = Map.of("content-type","application/octet-stream","topic",producerTopic,"X-Correlation-ID", correlationId);
        final WebClient.RequestHeadersSpec<?> request = client.post().uri(sidecarUrl+"/pubsub/publish")
                .headers(httpHeaders -> httpHeaders.setAll(headersMap))
                .body(BodyInserters.fromValue(payload));
        var resp = request.exchange().delaySubscription(Duration.ofMillis(2)).subscribe();

    }

    public void subscribeForMessagesFromSidecar(String clientUrl, String topic, String clientId,String correlationId) throws JsonProcessingException {
        ObjectMapper om = new ObjectMapper();
        var p = new ClientSubscribeRequest().setClientTopic(topic).setEndpoint(new EndpointDescription().setUrl(clientUrl).setClientId(clientId));
        logger.info("subscribe request {}",p);
        final Map<String, String> headersMap = Map.of("content-type","application/json","topic",topic,"X-Correlation-ID", correlationId);
        final WebClient.RequestHeadersSpec<?> request = client.post().uri(sidecarUrl+"/pubsub/subscribe")
                .headers(httpHeaders -> httpHeaders.setAll(headersMap))
                .body(BodyInserters.fromValue(om.writeValueAsBytes(p)));

        var resp = request.exchange().block();
        var body = resp.bodyToMono(String.class).block();
        logger.info(String.format("Subscription status = %s , %s", resp.statusCode(),body));
    }

}
