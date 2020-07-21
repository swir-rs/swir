
package rs.swir.api.server;
import com.google.protobuf.ByteString;
import io.grpc.Context;
import io.grpc.Server;
import io.grpc.ServerBuilder;
import rs.swir.api.client.*;


import java.io.IOException;
import java.util.concurrent.*;


import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class GrpcServer   {
    final ExecutorService ex = Executors.newCachedThreadPool();
    private static final Logger logger = LoggerFactory.getLogger(GrpcServer.class.getName());


    private final int port;
    private final Server server;


    public GrpcServer(int port) throws IOException {
        this(ServerBuilder.forPort(port), port);
    }

    /** Create a RouteGuide server using serverBuilder as a base and features as data. */
    public GrpcServer(ServerBuilder<?> serverBuilder, int port) {
        this.port = port;
        server = serverBuilder.addService(new GrpcServerService())
                .build();
    }

    /** Start serving requests. */
    public void start() throws IOException {
        server.start();
        logger.info("Server started, listening on " + port);
        Runtime.getRuntime().addShutdownHook(new Thread() {
            @Override
            public void run() {
                // Use stderr here since the logger may have been reset by its JVM shutdown hook.
                System.err.println("*** shutting down gRPC server since JVM is shutting down");
                try {
                    GrpcServer.this.stop();
                } catch (InterruptedException e) {
                    e.printStackTrace(System.err);
                }
                System.err.println("*** server shut down");
            }
        });
    }

    /** Stop serving requests and shutdown resources. */
    public void stop() throws InterruptedException {
        if (server != null) {
            server.shutdown().awaitTermination(30, TimeUnit.SECONDS);
        }
    }

    /**
     * Await termination on the main thread since the grpc library uses daemon threads.
     */
    private void blockUntilShutdown() throws InterruptedException {
        if (server != null) {
            server.awaitTermination();
        }
    }

    /**
     * Main method.  This comment makes the linter happy.
     */
    public static void main(String[] args) throws Exception {
        var port = Integer.parseInt(System.getenv("grpc_port"));
        GrpcServer server = new GrpcServer(port);
        server.start();
        server.blockUntilShutdown();
    }


    private static class GrpcServerService extends PubSubApiGrpc.PubSubApiImplBase{

        private BlockingQueue queue = new ArrayBlockingQueue<GrpcEvent>(1000, true);

        public void publish(rs.swir.api.client.PublishRequest request,
                            io.grpc.stub.StreamObserver<rs.swir.api.client.PublishResponse> responseObserver) {
            PublishResponse pr = PublishResponse.newBuilder().setStatus("GRPC is good").build();
            try {
                queue.put(new GrpcEvent().setPayload(request.getPayload().toByteArray()));
                responseObserver.onNext(pr);
            }catch(Exception ex) {
                logger.warn(ex.getMessage(),ex);
                responseObserver.onError(ex);
            }finally{
                responseObserver.onCompleted();
            }
        }

        public void subscribe(rs.swir.api.client.SubscribeRequest request,
                              io.grpc.stub.StreamObserver<rs.swir.api.client.SubscribeResponse> responseObserver) {

            try {
                while(!Context.current().isCancelled()) {
                    var e = (GrpcEvent)queue.poll();
                    if(e!=null) {
                        var sr = SubscribeResponse.newBuilder().setPayload(ByteString.copyFrom(e.getPayload())).build();
                        responseObserver.onNext(sr);
                    }
                }
                logger.info("Context got cancelled");
            }catch(Exception ex) {
                logger.warn(ex.getMessage(),ex);
                responseObserver.onError(ex);
            }finally{
                logger.warn("Leaving");
                responseObserver.onCompleted();
            }

        }
    }

}
