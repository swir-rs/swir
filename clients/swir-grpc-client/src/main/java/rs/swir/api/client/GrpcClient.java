
package rs.swir.api.client;

import com.google.protobuf.ByteString;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.StatusRuntimeException;

import java.nio.charset.Charset;
import java.util.concurrent.TimeUnit;
import java.util.logging.Level;
import java.util.logging.Logger;

public class GrpcClient {
    private static final Logger logger = Logger.getLogger(GrpcClient.class.getName());

    private final ManagedChannel channel;
    private final ClientApiGrpc.ClientApiBlockingStub blockingStub;

    /** Construct client connecting to HelloWorld server at {@code host:port}. */
    public GrpcClient(String host, int port) {
        this(ManagedChannelBuilder.forAddress(host, port)
                // Channels are secure by default (via SSL/TLS). For the example we disable TLS to avoid
                // needing certificates.
                .usePlaintext()
                .build());
    }

    /** Construct client for accessing HelloWorld server using the existing channel. */
    GrpcClient(ManagedChannel channel) {
        this.channel = channel;
        blockingStub = ClientApiGrpc.newBlockingStub(channel);
    }

    public void shutdown() throws InterruptedException {
        channel.shutdown().awaitTermination(5, TimeUnit.SECONDS);
    }

    /** Say hello to server. */
    public void publish(String payload) {
        logger.info("Publishing to broker" + payload);
        PublishRequest request = PublishRequest.newBuilder().setTopic("sometopic").setPayload(ByteString.copyFrom("hellopayload", Charset.forName("UTF-8"))).build();
        PublishResponse response;
        try {
            response = blockingStub.publish(request);
        } catch (StatusRuntimeException e) {
            logger.log(Level.WARNING, "RPC failed: {0}", e.getStatus());
            return;
        }
        logger.info("Response status: " + response.getStatus());
    }

    /**
     * Greet server. If provided, the first element of {@code args} is the name to use in the
     * greeting.
     */
    public static void main(String[] args) throws Exception {
        // Access a service running on the local machine on port 50051
        GrpcClient client = new GrpcClient("[::1]", 50051);
        try {
            String payload = "beer is good ";
            // Use the arg as the name to greet if provided
            if (args.length > 0) {
                payload = args[0];
            }
            client.publish(payload);
        } finally {
            client.shutdown();
        }
    }
}
