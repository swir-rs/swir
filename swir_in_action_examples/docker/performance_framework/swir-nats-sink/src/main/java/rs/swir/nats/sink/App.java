package rs.swir.nats.sink;

import io.nats.client.Connection;
import io.nats.client.Dispatcher;
import io.nats.client.Nats;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.nio.charset.StandardCharsets;
import java.time.Duration;

public class App {
    private static final Logger logger
            = LoggerFactory.getLogger(App.class);

    public static void main(String[] args) {
        String bootstrapServers = System.getenv("bootstrap_servers");
        String subscribeTopic = System.getenv("subscribe_topic");
        String responseTopic = System.getenv("response_topic");
        Connection nc=null;
        try {
            nc = Nats.connect(bootstrapServers);
            Connection finalNc = nc;
            Dispatcher d = nc.createDispatcher((msg) -> {
                logger.info(String.format("Received message \"%s\" on subject \"%s\", replying to %s\n",
                        new String(msg.getData(), StandardCharsets.UTF_8),
                        msg.getSubject(), responseTopic));
                finalNc.publish(responseTopic, msg.getData());

            });
            d.subscribe(subscribeTopic);
            nc.flush(Duration.ofMillis(500));
            while (true) {
                Thread.sleep(1000);
            }

        } catch (Exception e) {
            logger.error(e.getMessage());
        }finally {
            if (nc!=null) {
                try {
                    nc.close();
                } catch (InterruptedException e) {
                    logger.error(e.getMessage());
                }
            }
        }



    }
}
