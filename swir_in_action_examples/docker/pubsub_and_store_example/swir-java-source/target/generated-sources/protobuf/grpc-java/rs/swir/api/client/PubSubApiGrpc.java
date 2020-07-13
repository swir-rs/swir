package rs.swir.api.client;

import static io.grpc.MethodDescriptor.generateFullMethodName;
import static io.grpc.stub.ClientCalls.asyncBidiStreamingCall;
import static io.grpc.stub.ClientCalls.asyncClientStreamingCall;
import static io.grpc.stub.ClientCalls.asyncServerStreamingCall;
import static io.grpc.stub.ClientCalls.asyncUnaryCall;
import static io.grpc.stub.ClientCalls.blockingServerStreamingCall;
import static io.grpc.stub.ClientCalls.blockingUnaryCall;
import static io.grpc.stub.ClientCalls.futureUnaryCall;
import static io.grpc.stub.ServerCalls.asyncBidiStreamingCall;
import static io.grpc.stub.ServerCalls.asyncClientStreamingCall;
import static io.grpc.stub.ServerCalls.asyncServerStreamingCall;
import static io.grpc.stub.ServerCalls.asyncUnaryCall;
import static io.grpc.stub.ServerCalls.asyncUnimplementedStreamingCall;
import static io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall;

/**
 * <pre>
 * The definition of API exposed by Sidecar.
 * </pre>
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler (version 1.30.0)",
    comments = "Source: client_api.proto")
public final class PubSubApiGrpc {

  private PubSubApiGrpc() {}

  public static final String SERVICE_NAME = "swir_public.PubSubApi";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<rs.swir.api.client.PublishRequest,
      rs.swir.api.client.PublishResponse> getPublishMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Publish",
      requestType = rs.swir.api.client.PublishRequest.class,
      responseType = rs.swir.api.client.PublishResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<rs.swir.api.client.PublishRequest,
      rs.swir.api.client.PublishResponse> getPublishMethod() {
    io.grpc.MethodDescriptor<rs.swir.api.client.PublishRequest, rs.swir.api.client.PublishResponse> getPublishMethod;
    if ((getPublishMethod = PubSubApiGrpc.getPublishMethod) == null) {
      synchronized (PubSubApiGrpc.class) {
        if ((getPublishMethod = PubSubApiGrpc.getPublishMethod) == null) {
          PubSubApiGrpc.getPublishMethod = getPublishMethod =
              io.grpc.MethodDescriptor.<rs.swir.api.client.PublishRequest, rs.swir.api.client.PublishResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Publish"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.PublishRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.PublishResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PubSubApiMethodDescriptorSupplier("Publish"))
              .build();
        }
      }
    }
    return getPublishMethod;
  }

  private static volatile io.grpc.MethodDescriptor<rs.swir.api.client.PublishRequest,
      rs.swir.api.client.PublishResponse> getPublishBiStreamMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "PublishBiStream",
      requestType = rs.swir.api.client.PublishRequest.class,
      responseType = rs.swir.api.client.PublishResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.BIDI_STREAMING)
  public static io.grpc.MethodDescriptor<rs.swir.api.client.PublishRequest,
      rs.swir.api.client.PublishResponse> getPublishBiStreamMethod() {
    io.grpc.MethodDescriptor<rs.swir.api.client.PublishRequest, rs.swir.api.client.PublishResponse> getPublishBiStreamMethod;
    if ((getPublishBiStreamMethod = PubSubApiGrpc.getPublishBiStreamMethod) == null) {
      synchronized (PubSubApiGrpc.class) {
        if ((getPublishBiStreamMethod = PubSubApiGrpc.getPublishBiStreamMethod) == null) {
          PubSubApiGrpc.getPublishBiStreamMethod = getPublishBiStreamMethod =
              io.grpc.MethodDescriptor.<rs.swir.api.client.PublishRequest, rs.swir.api.client.PublishResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.BIDI_STREAMING)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "PublishBiStream"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.PublishRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.PublishResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PubSubApiMethodDescriptorSupplier("PublishBiStream"))
              .build();
        }
      }
    }
    return getPublishBiStreamMethod;
  }

  private static volatile io.grpc.MethodDescriptor<rs.swir.api.client.SubscribeRequest,
      rs.swir.api.client.SubscribeResponse> getSubscribeMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Subscribe",
      requestType = rs.swir.api.client.SubscribeRequest.class,
      responseType = rs.swir.api.client.SubscribeResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
  public static io.grpc.MethodDescriptor<rs.swir.api.client.SubscribeRequest,
      rs.swir.api.client.SubscribeResponse> getSubscribeMethod() {
    io.grpc.MethodDescriptor<rs.swir.api.client.SubscribeRequest, rs.swir.api.client.SubscribeResponse> getSubscribeMethod;
    if ((getSubscribeMethod = PubSubApiGrpc.getSubscribeMethod) == null) {
      synchronized (PubSubApiGrpc.class) {
        if ((getSubscribeMethod = PubSubApiGrpc.getSubscribeMethod) == null) {
          PubSubApiGrpc.getSubscribeMethod = getSubscribeMethod =
              io.grpc.MethodDescriptor.<rs.swir.api.client.SubscribeRequest, rs.swir.api.client.SubscribeResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Subscribe"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.SubscribeRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.SubscribeResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PubSubApiMethodDescriptorSupplier("Subscribe"))
              .build();
        }
      }
    }
    return getSubscribeMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static PubSubApiStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<PubSubApiStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<PubSubApiStub>() {
        @java.lang.Override
        public PubSubApiStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new PubSubApiStub(channel, callOptions);
        }
      };
    return PubSubApiStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static PubSubApiBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<PubSubApiBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<PubSubApiBlockingStub>() {
        @java.lang.Override
        public PubSubApiBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new PubSubApiBlockingStub(channel, callOptions);
        }
      };
    return PubSubApiBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static PubSubApiFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<PubSubApiFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<PubSubApiFutureStub>() {
        @java.lang.Override
        public PubSubApiFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new PubSubApiFutureStub(channel, callOptions);
        }
      };
    return PubSubApiFutureStub.newStub(factory, channel);
  }

  /**
   * <pre>
   * The definition of API exposed by Sidecar.
   * </pre>
   */
  public static abstract class PubSubApiImplBase implements io.grpc.BindableService {

    /**
     */
    public void publish(rs.swir.api.client.PublishRequest request,
        io.grpc.stub.StreamObserver<rs.swir.api.client.PublishResponse> responseObserver) {
      asyncUnimplementedUnaryCall(getPublishMethod(), responseObserver);
    }

    /**
     */
    public io.grpc.stub.StreamObserver<rs.swir.api.client.PublishRequest> publishBiStream(
        io.grpc.stub.StreamObserver<rs.swir.api.client.PublishResponse> responseObserver) {
      return asyncUnimplementedStreamingCall(getPublishBiStreamMethod(), responseObserver);
    }

    /**
     */
    public void subscribe(rs.swir.api.client.SubscribeRequest request,
        io.grpc.stub.StreamObserver<rs.swir.api.client.SubscribeResponse> responseObserver) {
      asyncUnimplementedUnaryCall(getSubscribeMethod(), responseObserver);
    }

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return io.grpc.ServerServiceDefinition.builder(getServiceDescriptor())
          .addMethod(
            getPublishMethod(),
            asyncUnaryCall(
              new MethodHandlers<
                rs.swir.api.client.PublishRequest,
                rs.swir.api.client.PublishResponse>(
                  this, METHODID_PUBLISH)))
          .addMethod(
            getPublishBiStreamMethod(),
            asyncBidiStreamingCall(
              new MethodHandlers<
                rs.swir.api.client.PublishRequest,
                rs.swir.api.client.PublishResponse>(
                  this, METHODID_PUBLISH_BI_STREAM)))
          .addMethod(
            getSubscribeMethod(),
            asyncServerStreamingCall(
              new MethodHandlers<
                rs.swir.api.client.SubscribeRequest,
                rs.swir.api.client.SubscribeResponse>(
                  this, METHODID_SUBSCRIBE)))
          .build();
    }
  }

  /**
   * <pre>
   * The definition of API exposed by Sidecar.
   * </pre>
   */
  public static final class PubSubApiStub extends io.grpc.stub.AbstractAsyncStub<PubSubApiStub> {
    private PubSubApiStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PubSubApiStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new PubSubApiStub(channel, callOptions);
    }

    /**
     */
    public void publish(rs.swir.api.client.PublishRequest request,
        io.grpc.stub.StreamObserver<rs.swir.api.client.PublishResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(getPublishMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public io.grpc.stub.StreamObserver<rs.swir.api.client.PublishRequest> publishBiStream(
        io.grpc.stub.StreamObserver<rs.swir.api.client.PublishResponse> responseObserver) {
      return asyncBidiStreamingCall(
          getChannel().newCall(getPublishBiStreamMethod(), getCallOptions()), responseObserver);
    }

    /**
     */
    public void subscribe(rs.swir.api.client.SubscribeRequest request,
        io.grpc.stub.StreamObserver<rs.swir.api.client.SubscribeResponse> responseObserver) {
      asyncServerStreamingCall(
          getChannel().newCall(getSubscribeMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   * <pre>
   * The definition of API exposed by Sidecar.
   * </pre>
   */
  public static final class PubSubApiBlockingStub extends io.grpc.stub.AbstractBlockingStub<PubSubApiBlockingStub> {
    private PubSubApiBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PubSubApiBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new PubSubApiBlockingStub(channel, callOptions);
    }

    /**
     */
    public rs.swir.api.client.PublishResponse publish(rs.swir.api.client.PublishRequest request) {
      return blockingUnaryCall(
          getChannel(), getPublishMethod(), getCallOptions(), request);
    }

    /**
     */
    public java.util.Iterator<rs.swir.api.client.SubscribeResponse> subscribe(
        rs.swir.api.client.SubscribeRequest request) {
      return blockingServerStreamingCall(
          getChannel(), getSubscribeMethod(), getCallOptions(), request);
    }
  }

  /**
   * <pre>
   * The definition of API exposed by Sidecar.
   * </pre>
   */
  public static final class PubSubApiFutureStub extends io.grpc.stub.AbstractFutureStub<PubSubApiFutureStub> {
    private PubSubApiFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PubSubApiFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new PubSubApiFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<rs.swir.api.client.PublishResponse> publish(
        rs.swir.api.client.PublishRequest request) {
      return futureUnaryCall(
          getChannel().newCall(getPublishMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_PUBLISH = 0;
  private static final int METHODID_SUBSCRIBE = 1;
  private static final int METHODID_PUBLISH_BI_STREAM = 2;

  private static final class MethodHandlers<Req, Resp> implements
      io.grpc.stub.ServerCalls.UnaryMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ServerStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ClientStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.BidiStreamingMethod<Req, Resp> {
    private final PubSubApiImplBase serviceImpl;
    private final int methodId;

    MethodHandlers(PubSubApiImplBase serviceImpl, int methodId) {
      this.serviceImpl = serviceImpl;
      this.methodId = methodId;
    }

    @java.lang.Override
    @java.lang.SuppressWarnings("unchecked")
    public void invoke(Req request, io.grpc.stub.StreamObserver<Resp> responseObserver) {
      switch (methodId) {
        case METHODID_PUBLISH:
          serviceImpl.publish((rs.swir.api.client.PublishRequest) request,
              (io.grpc.stub.StreamObserver<rs.swir.api.client.PublishResponse>) responseObserver);
          break;
        case METHODID_SUBSCRIBE:
          serviceImpl.subscribe((rs.swir.api.client.SubscribeRequest) request,
              (io.grpc.stub.StreamObserver<rs.swir.api.client.SubscribeResponse>) responseObserver);
          break;
        default:
          throw new AssertionError();
      }
    }

    @java.lang.Override
    @java.lang.SuppressWarnings("unchecked")
    public io.grpc.stub.StreamObserver<Req> invoke(
        io.grpc.stub.StreamObserver<Resp> responseObserver) {
      switch (methodId) {
        case METHODID_PUBLISH_BI_STREAM:
          return (io.grpc.stub.StreamObserver<Req>) serviceImpl.publishBiStream(
              (io.grpc.stub.StreamObserver<rs.swir.api.client.PublishResponse>) responseObserver);
        default:
          throw new AssertionError();
      }
    }
  }

  private static abstract class PubSubApiBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    PubSubApiBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return rs.swir.api.client.SwirClientApiProto.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("PubSubApi");
    }
  }

  private static final class PubSubApiFileDescriptorSupplier
      extends PubSubApiBaseDescriptorSupplier {
    PubSubApiFileDescriptorSupplier() {}
  }

  private static final class PubSubApiMethodDescriptorSupplier
      extends PubSubApiBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final String methodName;

    PubSubApiMethodDescriptorSupplier(String methodName) {
      this.methodName = methodName;
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.MethodDescriptor getMethodDescriptor() {
      return getServiceDescriptor().findMethodByName(methodName);
    }
  }

  private static volatile io.grpc.ServiceDescriptor serviceDescriptor;

  public static io.grpc.ServiceDescriptor getServiceDescriptor() {
    io.grpc.ServiceDescriptor result = serviceDescriptor;
    if (result == null) {
      synchronized (PubSubApiGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new PubSubApiFileDescriptorSupplier())
              .addMethod(getPublishMethod())
              .addMethod(getPublishBiStreamMethod())
              .addMethod(getSubscribeMethod())
              .build();
        }
      }
    }
    return result;
  }
}
