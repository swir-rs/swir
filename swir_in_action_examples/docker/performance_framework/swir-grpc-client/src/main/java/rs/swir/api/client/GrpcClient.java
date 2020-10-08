
package rs.swir.api.client;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.google.protobuf.ByteString;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import io.grpc.ServerBuilder;
import io.grpc.StatusRuntimeException;
import io.grpc.stub.StreamObserver;
import rs.swir.api.client.payload.Payload;


import java.util.*;

import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;


public class GrpcClient {
    final ExecutorService ex = Executors.newCachedThreadPool();
    private static final Logger logger = LoggerFactory.getLogger(GrpcClient.class.getName());

    private final ManagedChannel channel;
    private final PubSubApiGrpc.PubSubApiBlockingStub blockingStub;
    private final PubSubApiGrpc.PubSubApiStub apiStub;
    private static final Random random = new Random();

    /**
     * Construct client connecting to HelloWorld server at {@code host:port}.
     */
    public GrpcClient(String host, int port) {
        this(ManagedChannelBuilder.forAddress(host, port)
                // Channels are secure by default (via SSL/TLS). For the example we disable TLS to avoid
                // needing certificates.
                .usePlaintext()
                .build());
    }

    /**
     * Construct client for accessing HelloWorld server using the existing channel.
     */
    GrpcClient(ManagedChannel channel) {
        this.channel = channel;
        blockingStub = PubSubApiGrpc.newBlockingStub(channel);
        apiStub = PubSubApiGrpc.newStub(channel);
    }

    public void shutdown() throws InterruptedException {
        channel.shutdown().awaitTermination(5, TimeUnit.SECONDS);
    }




    public static void main(String[] args) throws Exception {
        logger.info(String.format("GrpcClient"));
        String sidecarHostname = System.getenv("sidecar_hostname");
        int sidecarPort = Integer.parseInt(System.getenv("sidecar_port"));

        var client = new GrpcClient(sidecarHostname, sidecarPort);


        try {
            client.executeTest(args);
        } finally {
            client.shutdown();
        }
        System.exit(1);
    }

    /**
     * Greet server. If provided, the first element of {@code args} is the name to use in the
     * greeting.
     */
    public void executeTest(String[] args) throws Exception {
        var processedCounter = new AtomicInteger(0);

        var server = ServerBuilder.forPort(50053).addService(new NotificationApiGrpc.NotificationApiImplBase() {
            @Override
            public void subscriptionNotification(SubscribeNotification request, StreamObserver<SubscribeResponse> responseObserver) {
                logger.debug(String.format("Got notification  %s", request));
                var sr = SubscribeResponse.newBuilder().setCorrelationId(request.getCorrelationId()).setStatus("Ok").build();
                responseObserver.onNext(sr);
                processedCounter.incrementAndGet();
                responseObserver.onCompleted();
            }
        }).build();
        server.start();



        var messages = Integer.parseInt(System.getenv("messages"));
        var threads = Integer.parseInt(System.getenv("threads"));
        var producerTopic= System.getenv("client_request_topic");
        var subscriberTopic = System.getenv("client_response_topic");

        String publishType;
        var pt = System.getenv("publish_type");
        if (pt == null) {
            publishType = "unary";
        } else {
            publishType = pt;

        }

        var missedPacketsThreshold = 150;

        var testStarted = new AtomicBoolean(false);

        var om = new ObjectMapper();

        if (messages % threads != 0) {
            ObjectNode response = om.createObjectNode();
            response.put("error", " messages doesn't divide by threads ");
            logger.warn("messages doesn't divide by threads");
            return;
        }
        int offset = messages / threads;

        logger.info(String.format("offset %d", offset));

        var sr = subscribeForMessagesFromSidecar(blockingStub,subscriberTopic);

        Thread.sleep(5000);

        testStarted.set(true);
        final AtomicLong totalSendTime = new AtomicLong(0);
        Semaphore semaphore = new Semaphore(threads);

        final AtomicInteger sentCount = new AtomicInteger(0);
        semaphore.acquire(threads);
        long totalStart = System.nanoTime();
        var streamObservers = new ArrayList<StreamObserver<PublishRequest>>();
        for (int i = 0; i < threads; i++) {
            final int k = i;
            ex.submit(new Runnable() {
                public void run() {

                    logger.info(String.format("Publish type %s", publishType));

                    switch (publishType) {
                        case "unary":
                            unaryMessagesToSidecar(apiStub, k, offset, om, producerTopic, subscriberTopic, sentCount, totalSendTime, semaphore);
                            break;

                        default:
                            logger.error(String.format("Don't know what to do with %s", publishType));
                    }


                }
            });
        }
        ;
        semaphore.acquire(threads);
        int oldCount = 0;
        int missingPacketCounter = 0;
        boolean missedPackets = false;

        while (sentCount.get() != messages) {
            Thread.sleep(100);
        }
        logger.info(String.format("sent count %d %d", sentCount.get(), messages));

        while (processedCounter.get() != (messages) && !missedPackets) {
            int count = processedCounter.get();
            logger.debug(String.format("completed count %d %d", count, oldCount));
            if (oldCount == count) {
                missingPacketCounter++;
                logger.warn(String.format("Count has not changed %d", missingPacketCounter));
            } else {
                missingPacketCounter = 0;
            }

            if (missingPacketCounter > missedPacketsThreshold) {
                missedPackets = true;
                logger.warn(String.format("Count has not changed %d", missingPacketCounter));
            }
            oldCount = count;
            Thread.sleep(100);
        }

        streamObservers.stream().filter(f -> f != null).forEach(f -> {
                    try {
                        logger.info(String.format("Closing connections %d",messages - processedCounter.get()));
                        f.onCompleted();
                    } catch (RuntimeException ex) {
                        logger.warn(ex.getMessage());
                    }catch (Exception e) {
                        logger.warn(e.getMessage());
                    }
                }
        );

        unsubscribeForMessagesFromSidecar(blockingStub,sr);

        long ts = totalSendTime.get() / threads;
        long totalEnd = System.nanoTime();
        long tt = totalEnd - totalStart;
        testStarted.set(false);
        ObjectNode response = om.createObjectNode();
        response.put("averageSendTimeNs", ts);
        response.put("averageSendTimeMs", TimeUnit.MILLISECONDS.convert(ts, TimeUnit.NANOSECONDS));
        response.put("averageSendTimeS", TimeUnit.SECONDS.convert(ts, TimeUnit.NANOSECONDS));
        response.put("totalTimeNs", tt);
        response.put("totalTimeMs", TimeUnit.MILLISECONDS.convert(tt, TimeUnit.NANOSECONDS));
        response.put("totalTimeS", TimeUnit.SECONDS.convert(tt, TimeUnit.NANOSECONDS));
        response.put("throughput msg/sec", ((double) messages / tt) * TimeUnit.NANOSECONDS.convert(1, TimeUnit.SECONDS));
        response.put("packets missed ", messages - processedCounter.get());
        logger.info(String.format("{%s}", response));


        ex.shutdown();

        return;
    }

    private SubscribeRequest subscribeForMessagesFromSidecar(PubSubApiGrpc.PubSubApiBlockingStub apiStub, String subscriberTopic) {
        var  sr = SubscribeRequest.newBuilder().setClientId("someclient").setCorrelationId(UUID.randomUUID().toString()).setTopic(subscriberTopic).build();
        logger.debug("Subscribe request {}",sr);
        var response = apiStub.subscribe(sr);
        logger.debug(String.format("Subscribe response status: %s", response.getStatus()));
        return sr;
    }

    private void unsubscribeForMessagesFromSidecar(PubSubApiGrpc.PubSubApiBlockingStub apiStub, SubscribeRequest subscribeRequest) {
        var  sr = UnsubscribeRequest.newBuilder().setClientId(subscribeRequest.getClientId()).setCorrelationId(subscribeRequest.getCorrelationId()).setTopic(subscribeRequest.getTopic()).build();
        logger.debug("Unsubscribe request {}",sr);
        var response = apiStub.unsubscribe(sr);
        logger.debug(String.format("Unsubscribe response status: %s", response.getStatus()));
    }


    void unaryMessagesToSidecar(PubSubApiGrpc.PubSubApiStub apiStub, int k, int offset, ObjectMapper om, String producer, String subscriber, AtomicInteger sentCount, AtomicLong totalSendTime, Semaphore semaphore) {
        logger.debug(String.format("Executing run %d ", k));
        long start = System.nanoTime();
        try {
            for (int j = 0; j < offset; j++) {
                final int c = k * offset + j;
                sendMessageToSidecar(blockingStub, c, om, producer, subscriber, sentCount);
            }
        } catch (Exception e) {
            logger.warn(e.getMessage(), e);
        } finally {
            long stop = System.nanoTime();
            totalSendTime.addAndGet((stop - start));
            logger.debug(String.format("Run %d completed in %d", k, (stop - start)));
            semaphore.release();
        }

    }

    void sendMessageToSidecar(PubSubApiGrpc.PubSubApiBlockingStub blockingStub, int c, ObjectMapper om, String producer,String subscriber, AtomicInteger sentCount) throws JsonProcessingException, ExecutionException, InterruptedException {
        byte[] bytes = new byte[64];
        random.nextBytes(bytes);;
        var p = new Payload().setProducer(producer).setConsumer(subscriber).setCounter(c).setTimestamp(System.currentTimeMillis()).setPayload(Base64.getEncoder().encodeToString(bytes));
        logger.debug(String.format("sending request %s", p));
        PublishRequest request = PublishRequest.newBuilder().setCorrelationId(UUID.randomUUID().toString()).setTopic(producer).setPayload(ByteString.copyFrom(om.writeValueAsBytes(p))).build();
        PublishResponse response;
        try {
            response = blockingStub.publish(request);
            sentCount.incrementAndGet();
            logger.debug(String.format("Response status: %s", response.getStatus()));
        } catch (StatusRuntimeException e) {
            logger.warn(String.format("RPC failed: %s %s", e.getMessage(), p), e);
            return;
        }

    }
}

