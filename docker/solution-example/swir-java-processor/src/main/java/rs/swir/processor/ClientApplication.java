package rs.swir.processor;

import io.netty.channel.ChannelOption;
import io.netty.handler.timeout.ReadTimeoutHandler;
import io.netty.handler.timeout.WriteTimeoutHandler;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.context.annotation.Bean;
import org.springframework.http.client.reactive.ClientHttpConnector;
import org.springframework.http.client.reactive.ReactorClientHttpConnector;
import org.springframework.web.reactive.function.client.WebClient;
import reactor.netty.http.client.HttpClient;

import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;


@SpringBootApplication
public class ClientApplication {

    private static Logger logger = LoggerFactory.getLogger(ClientApplication.class);

    @Bean
    public WebClient produceWebClient() {
        int connectTimeOut = 5000;
        int readTimeOut = 20;
        long writeTimeOut = 20;

        HttpClient httpClient = HttpClient.create().tcpConfiguration(tcpClient -> {
            tcpClient = tcpClient.option(ChannelOption.CONNECT_TIMEOUT_MILLIS, connectTimeOut);

            tcpClient = tcpClient.doOnConnected(conn -> conn
                    .addHandlerLast(new ReadTimeoutHandler(readTimeOut, TimeUnit.SECONDS))
                    .addHandlerLast(new WriteTimeoutHandler(writeTimeOut, TimeUnit.SECONDS))
            );
            return tcpClient;
        });

        ClientHttpConnector connector = new ReactorClientHttpConnector(httpClient);
        return WebClient.builder().clientConnector(connector).build();
    }

    @Bean
    public AtomicInteger produceExpectedMessagesCounter() {
        return new AtomicInteger();
    }

    @Bean
    public AtomicBoolean produceTestStatus() {
        return new AtomicBoolean();
    }


    public static void main(String[] args) {
        SpringApplication.run(ClientApplication.class, args);
        logger.info("Args = {}", (Object) args);

    }
}
