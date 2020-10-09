
package rs.swir.api.client;


import com.fasterxml.jackson.databind.ObjectMapper;

import com.google.protobuf.ByteString;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.StatusRuntimeException;
import rs.swir.api.client.payload.Payload;
import java.util.*;
import java.util.concurrent.*;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;


public class GrpcClient {
    final ExecutorService ex = Executors.newCachedThreadPool();
    private static final Logger logger = LoggerFactory.getLogger(GrpcClient.class.getName());

    private final ManagedChannel channel;
    private final PubSubApiGrpc.PubSubApiBlockingStub blockingStub;
    private final PubSubApiGrpc.PubSubApiStub apiStub;
    private String clientDatabaseName = "";
    private final PersistenceApiGrpc.PersistenceApiBlockingStub persistenceStub;

    private static final Random random = new Random();

    /**
     * Construct client connecting to HelloWorld server at {@code host:port}.
     */
    public GrpcClient(String host, int port,String clientDatabaseName) {
        this(ManagedChannelBuilder.forAddress(host, port)
                // Channels are secure by default (via SSL/TLS). For the example we disable TLS to avoid
                // needing certificates.
                .usePlaintext()
                .build());
        this.clientDatabaseName = clientDatabaseName;
    }

    /**
     * Construct client for accessing HelloWorld server using the existing channel.
     */
    GrpcClient(ManagedChannel channel) {
        this.channel = channel;
        blockingStub = PubSubApiGrpc.newBlockingStub(channel);
        apiStub = PubSubApiGrpc.newStub(channel);
        persistenceStub = PersistenceApiGrpc.newBlockingStub(channel);

    }

    public void shutdown() throws InterruptedException {
        channel.shutdown().awaitTermination(5, TimeUnit.SECONDS);
    }

    public static void main(String[] args) throws Exception {
        logger.info(String.format("GrpcClient"));
        var sidecarHostname = System.getenv("sidecar_hostname");
        var sidecarPort = Integer.parseInt(System.getenv("sidecar_port"));
        var clientDatabaseName = System.getenv("client_database_name");

        var client = new GrpcClient(sidecarHostname, sidecarPort,clientDatabaseName);
        try {
            client.run(args);
        } finally {
            client.shutdown();
        }
        System.exit(1);
    }

    /**
     * Greet server. If provided, the first element of {@code args} is the name to use in the
     * greeting.
     */
    public void run(String[] args) throws Exception {
        var producerTopic= System.getenv("produce_topic");
        var om = new ObjectMapper();
        ex.execute(() -> {
            biStreamMessagesToSidecar(apiStub, persistenceStub, om, producerTopic);
        });
        while(true){
            Thread.sleep(1000);
        }
    }


    void  biStreamMessagesToSidecar(PubSubApiGrpc.PubSubApiStub apiStub, PersistenceApiGrpc.PersistenceApiBlockingStub persistenceApiStub, ObjectMapper om, String producer) {

        try {

            var persistenceObserver = new io.grpc.stub.StreamObserver<rs.swir.api.client.StoreResponse>() {
                @Override
                public void onNext(StoreResponse value) {
                    logger.debug(String.format("Response status: %s", value.getStatus()));
                }

                @Override
                public void onError(Throwable t) {
                    logger.warn(String.format("Server thrown an error %s" , t.getMessage()), t);
                }

                @Override
                public void onCompleted() {
                    logger.info(String.format("Persistence Server sent completed"));
                }
            };

            var counter = 0;
            while(true){
                Thread.sleep(1000);
                var bytes = new byte[64];
                random.nextBytes(bytes);;
                var correlationId = UUID.randomUUID().toString();
		        int c = counter++;
                var p = new Payload().setProducer(producer).setConsumer(producer).setCounter(c).setTimestamp(System.currentTimeMillis()).setPayload(Base64.getEncoder().encodeToString(bytes));
                logger.info(String.format("produced : %s", p));
                var request = PublishRequest.newBuilder().setCorrelationId(correlationId).setTopic(producer).setPayload(ByteString.copyFrom(om.writeValueAsBytes(p))).build();
                var res = blockingStub.publish(request);
                var sr = StoreRequest.newBuilder().setCorrelationId(correlationId).setDatabaseName(clientDatabaseName).setKey(Integer.toString(c)).setPayload(ByteString.copyFrom(om.writeValueAsBytes(p))).build();
                var result = persistenceApiStub.store(sr);
                logger.info(String.format("stored : %s %s",p,  result.getStatus()));

            }
        } catch (StatusRuntimeException e) {
            logger.warn(String.format("RPC failed: %s ", e.getMessage()), e);
        } catch (Exception ex) {
            logger.warn(String.format("RPC failed: %s ", ex.getMessage()), ex);
        }
    }
}

