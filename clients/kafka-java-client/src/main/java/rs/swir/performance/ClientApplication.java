package rs.swir.performance;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.context.annotation.Bean;
import org.springframework.web.client.RestTemplate;

import java.util.concurrent.atomic.AtomicInteger;


@SpringBootApplication
public class ClientApplication {

	private static Logger logger = LoggerFactory.getLogger(ClientApplication.class);


	@Bean
	public AtomicInteger produceExpectedMessagesCounter(){
		return new AtomicInteger();
	}


	@Bean
	public RestTemplate produceRestTemplate(){
		return new RestTemplate();
	}

	public static void main(String[] args) {
		SpringApplication.run(ClientApplication.class, args);
		logger.info("Args = {}",(Object)args);

	}
}
