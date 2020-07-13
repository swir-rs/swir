
package rs.swir.api.client;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.google.protobuf.ByteString;
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
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


    public void subscribeForMessagesFromSidecar(final String topic, final AtomicInteger processedCounter) {
        ex.submit(() -> {
            logger.info(String.format("Subscribing to topic %s", topic));
            SubscribeRequest request = SubscribeRequest.newBuilder().setCorrelationId(UUID.randomUUID().toString()).setTopic(topic).build();


            apiStub.subscribe(request, new io.grpc.stub.StreamObserver<>() {
                @Override
                public void onNext(SubscribeResponse sr) {
                    var s = new String(sr.getPayload().toByteArray());
                    logger.debug(String.format("Message from Sidecar %s", s));
                    int count = processedCounter.incrementAndGet();
                    if (count % 1000 == 0) {
                        logger.info(String.format("Received  %s", count));
                    }
                }

                @Override
                public void onError(Throwable t) {
                    logger.warn("Subscribe stream error " + t.getMessage());
                }

                @Override
                public void onCompleted() {
                    logger.info("Subscribe stream completed");
                }
            });
        });
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

        var processedCounter = new AtomicInteger(0);
        var testStarted = new AtomicBoolean(false);
        processedCounter.set(0);

        var om = new ObjectMapper();

        if (messages % threads != 0) {
            ObjectNode response = om.createObjectNode();
            response.put("error", " messages doesn't divide by threads ");
            logger.warn("messages doesn't divide by threads");
            return;
        }
        int offset = messages / threads;

        logger.info(String.format("offset %d", offset));

        subscribeForMessagesFromSidecar(subscriberTopic, processedCounter);

        Thread.sleep(5000);

        testStarted.set(true);
        final AtomicLong totalSendTime = new AtomicLong(0);
        Semaphore semaphore = new Semaphore(threads);

        final AtomicInteger sentCount = new AtomicInteger();
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
//                        case "uni":
//                            streamMessagesToSidecar(apiStub, k, offset, om, producerTopics, sentCount, totalSendTime, semaphore);
//                            break;
                        case "bidi":
                            streamObservers.add(biStreamMessagesToSidecar(apiStub, k, offset, om, producerTopic, subscriberTopic, sentCount, totalSendTime, semaphore));
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


    io.grpc.stub.StreamObserver<rs.swir.api.client.PublishRequest> biStreamMessagesToSidecar(PubSubApiGrpc.PubSubApiStub apiStub, int k, int offset, ObjectMapper om, String producer, String consumer, AtomicInteger sentCount, AtomicLong totalSendTime, Semaphore semaphore) {
        var responses = new AtomicInteger(0);

        try {
            var responseObserver = new io.grpc.stub.StreamObserver<rs.swir.api.client.PublishResponse>() {
                @Override
                public void onNext(PublishResponse value) {
                    logger.debug(String.format("Response status: %s", value.getStatus()));
                    responses.incrementAndGet();
                }

                @Override
                public void onError(Throwable t) {
                    logger.warn(String.format("Server thrown an error %s %d", t.getMessage(),responses.get()), t);
                }

                @Override
                public void onCompleted() {
                    logger.info(String.format("Server sent completed %d",responses.get()));
                }
            };
            var response = apiStub.publishBiStream(responseObserver);
            logger.debug(String.format("Executing run %d ", k));
            long start = System.nanoTime();
            try {
                for (int j = 0; j < offset; j++) {
                    final int c = k * offset + j;
                    byte[] bytes = new byte[64];
                    random.nextBytes(bytes);;
                    var p = new Payload().setProducer(producer).setConsumer(consumer).setCounter(c).setTimestamp(System.currentTimeMillis()).setPayload(Base64.getEncoder().encodeToString(bytes));
                    logger.debug(String.format("sending request %s", p));
                    PublishRequest request = PublishRequest.newBuilder().setCorrelationId(UUID.randomUUID().toString()).setTopic(producer).setPayload(ByteString.copyFrom(om.writeValueAsBytes(p))).build();
                    response.onNext(request);
                    sentCount.incrementAndGet();
                }
            } catch (Exception e) {
                logger.warn(e.getMessage(), e);
            } finally {
                long stop = System.nanoTime();
                totalSendTime.addAndGet((stop - start));
                logger.debug(String.format("Run %d completed in %d", k, (stop - start)));
                semaphore.release();
            }
            return response;

        } catch (StatusRuntimeException e) {
            logger.warn(String.format("RPC failed: %s ", e.getMessage()), e);
            return null;
        } catch (Exception ex) {
            logger.warn(String.format("RPC failed: %s ", ex.getMessage()), ex);
            return null;
        }

    }


//    void streamMessagesToSidecar(ClientApiGrpc.ClientApiStub apiStub, int k, int offset, ObjectMapper om, String clientTopic, AtomicInteger sentCount, AtomicLong totalSendTime, Semaphore semaphore) {
//        try {
//            var responseObserver = new io.grpc.stub.StreamObserver<rs.swir.api.client.PublishResponse>() {
//                @Override
//                public void onNext(PublishResponse value) {
//                    logger.debug(String.format("Response status: %s", value.getStatus()));
//                }
//
//                @Override
//                public void onError(Throwable t) {
//                    logger.warn(String.format("Server thrown an error %s",t.getMessage()), t);
//                }
//
//                @Override
//                public void onCompleted() {
//                    logger.info(String.format("Server sent completed "));
//                }
//            };
//            var response = apiStub.publishStream(responseObserver);
//            logger.debug(String.format("Executing run %d ", k));
//            long start = System.nanoTime();
//            try {
//                for (int j = 0; j < offset; j++) {
//                    final int c = k * offset + j;
//                    var p = new Payload().setName("client").setSurname("fooo").setCounter(c);
//                    logger.debug(String.format("sending request %s", p));
//                    PublishRequest request = PublishRequest.newBuilder().setTopic(clientTopic).setPayload(ByteString.copyFrom(om.writeValueAsBytes(p))).build();
//                    response.onNext(request);
//                    sentCount.incrementAndGet();
//                }
//            } catch (Exception e) {
//                logger.warn(e.getMessage(), e);
//            } finally {
////                response.onCompleted();
//                long stop = System.nanoTime();
//                totalSendTime.addAndGet((stop - start));
//                logger.debug(String.format("Run %d completed in %d", k, (stop - start)));
//                semaphore.release();
//            }
//
//        } catch (StatusRuntimeException e) {
//            logger.warn(String.format("RPC failed: %s ", e.getMessage()), e);
//            return;
//        } catch (Exception ex) {
//            logger.warn(String.format("RPC failed: %s ", ex.getMessage()), ex);
//            return;
//        }
//    }

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

