package io.swir.client;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.swir.client.payload.Payload;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.context.annotation.Bean;
import org.springframework.http.HttpEntity;
import org.springframework.http.ResponseEntity;
import org.springframework.web.client.RestTemplate;

import java.util.concurrent.TimeUnit;

@SpringBootApplication
public class ClientApplication {

	private static Logger logger = LoggerFactory.getLogger(ClientApplication.class);


	@Bean
	public RestTemplate produceRestTemplate(){
		return new RestTemplate();
	}

	public static void main(String[] args) {
		SpringApplication.run(ClientApplication.class, args);
		logger.info("Args = {}",(Object)args);
//		RestTemplate restTemplate = new RestTemplate();
//		String publishUrl
//				= "http://"+args[0]+"/publish";
//
//		ObjectMapper om = new ObjectMapper();
//
//		long start = System.nanoTime();
//		int messages = 1000;
//		for(int i = 0; i <messages; i++) {
//			HttpEntity<Payload> request = new HttpEntity<>(new Payload().setName("bar").setSurname("fooo").setCounter(i));
//			ResponseEntity<String> response = restTemplate.postForEntity(publishUrl, request, String.class);
//			if (!response.getStatusCode().is2xxSuccessful()) {
//				logger.warn("Got invalid response {}", response.getStatusCode());
//			}
//		}
//		long stop = System.nanoTime();
//		logger.info("Done  {} ms throughput {} messages/sec", TimeUnit.MILLISECONDS.convert(stop-start,TimeUnit.NANOSECONDS), messages/TimeUnit.SECONDS.convert(stop-start,TimeUnit.NANOSECONDS));

	}
}
