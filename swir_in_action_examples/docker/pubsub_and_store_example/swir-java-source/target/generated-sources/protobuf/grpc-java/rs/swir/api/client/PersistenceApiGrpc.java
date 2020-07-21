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
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler (version 1.30.0)",
    comments = "Source: client_api.proto")
public final class PersistenceApiGrpc {

  private PersistenceApiGrpc() {}

  public static final String SERVICE_NAME = "swir_public.PersistenceApi";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<rs.swir.api.client.StoreRequest,
      rs.swir.api.client.StoreResponse> getStoreMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Store",
      requestType = rs.swir.api.client.StoreRequest.class,
      responseType = rs.swir.api.client.StoreResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<rs.swir.api.client.StoreRequest,
      rs.swir.api.client.StoreResponse> getStoreMethod() {
    io.grpc.MethodDescriptor<rs.swir.api.client.StoreRequest, rs.swir.api.client.StoreResponse> getStoreMethod;
    if ((getStoreMethod = PersistenceApiGrpc.getStoreMethod) == null) {
      synchronized (PersistenceApiGrpc.class) {
        if ((getStoreMethod = PersistenceApiGrpc.getStoreMethod) == null) {
          PersistenceApiGrpc.getStoreMethod = getStoreMethod =
              io.grpc.MethodDescriptor.<rs.swir.api.client.StoreRequest, rs.swir.api.client.StoreResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Store"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.StoreRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.StoreResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PersistenceApiMethodDescriptorSupplier("Store"))
              .build();
        }
      }
    }
    return getStoreMethod;
  }

  private static volatile io.grpc.MethodDescriptor<rs.swir.api.client.RetrieveRequest,
      rs.swir.api.client.RetrieveResponse> getRetrieveMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Retrieve",
      requestType = rs.swir.api.client.RetrieveRequest.class,
      responseType = rs.swir.api.client.RetrieveResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<rs.swir.api.client.RetrieveRequest,
      rs.swir.api.client.RetrieveResponse> getRetrieveMethod() {
    io.grpc.MethodDescriptor<rs.swir.api.client.RetrieveRequest, rs.swir.api.client.RetrieveResponse> getRetrieveMethod;
    if ((getRetrieveMethod = PersistenceApiGrpc.getRetrieveMethod) == null) {
      synchronized (PersistenceApiGrpc.class) {
        if ((getRetrieveMethod = PersistenceApiGrpc.getRetrieveMethod) == null) {
          PersistenceApiGrpc.getRetrieveMethod = getRetrieveMethod =
              io.grpc.MethodDescriptor.<rs.swir.api.client.RetrieveRequest, rs.swir.api.client.RetrieveResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Retrieve"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.RetrieveRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.RetrieveResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PersistenceApiMethodDescriptorSupplier("Retrieve"))
              .build();
        }
      }
    }
    return getRetrieveMethod;
  }

  private static volatile io.grpc.MethodDescriptor<rs.swir.api.client.DeleteRequest,
      rs.swir.api.client.DeleteResponse> getDeleteMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Delete",
      requestType = rs.swir.api.client.DeleteRequest.class,
      responseType = rs.swir.api.client.DeleteResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<rs.swir.api.client.DeleteRequest,
      rs.swir.api.client.DeleteResponse> getDeleteMethod() {
    io.grpc.MethodDescriptor<rs.swir.api.client.DeleteRequest, rs.swir.api.client.DeleteResponse> getDeleteMethod;
    if ((getDeleteMethod = PersistenceApiGrpc.getDeleteMethod) == null) {
      synchronized (PersistenceApiGrpc.class) {
        if ((getDeleteMethod = PersistenceApiGrpc.getDeleteMethod) == null) {
          PersistenceApiGrpc.getDeleteMethod = getDeleteMethod =
              io.grpc.MethodDescriptor.<rs.swir.api.client.DeleteRequest, rs.swir.api.client.DeleteResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Delete"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.DeleteRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  rs.swir.api.client.DeleteResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PersistenceApiMethodDescriptorSupplier("Delete"))
              .build();
        }
      }
    }
    return getDeleteMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static PersistenceApiStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<PersistenceApiStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<PersistenceApiStub>() {
        @java.lang.Override
        public PersistenceApiStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new PersistenceApiStub(channel, callOptions);
        }
      };
    return PersistenceApiStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static PersistenceApiBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<PersistenceApiBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<PersistenceApiBlockingStub>() {
        @java.lang.Override
        public PersistenceApiBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new PersistenceApiBlockingStub(channel, callOptions);
        }
      };
    return PersistenceApiBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static PersistenceApiFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<PersistenceApiFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<PersistenceApiFutureStub>() {
        @java.lang.Override
        public PersistenceApiFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new PersistenceApiFutureStub(channel, callOptions);
        }
      };
    return PersistenceApiFutureStub.newStub(factory, channel);
  }

  /**
   */
  public static abstract class PersistenceApiImplBase implements io.grpc.BindableService {

    /**
     */
    public void store(rs.swir.api.client.StoreRequest request,
        io.grpc.stub.StreamObserver<rs.swir.api.client.StoreResponse> responseObserver) {
      asyncUnimplementedUnaryCall(getStoreMethod(), responseObserver);
    }

    /**
     */
    public void retrieve(rs.swir.api.client.RetrieveRequest request,
        io.grpc.stub.StreamObserver<rs.swir.api.client.RetrieveResponse> responseObserver) {
      asyncUnimplementedUnaryCall(getRetrieveMethod(), responseObserver);
    }

    /**
     */
    public void delete(rs.swir.api.client.DeleteRequest request,
        io.grpc.stub.StreamObserver<rs.swir.api.client.DeleteResponse> responseObserver) {
      asyncUnimplementedUnaryCall(getDeleteMethod(), responseObserver);
    }

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return io.grpc.ServerServiceDefinition.builder(getServiceDescriptor())
          .addMethod(
            getStoreMethod(),
            asyncUnaryCall(
              new MethodHandlers<
                rs.swir.api.client.StoreRequest,
                rs.swir.api.client.StoreResponse>(
                  this, METHODID_STORE)))
          .addMethod(
            getRetrieveMethod(),
            asyncUnaryCall(
              new MethodHandlers<
                rs.swir.api.client.RetrieveRequest,
                rs.swir.api.client.RetrieveResponse>(
                  this, METHODID_RETRIEVE)))
          .addMethod(
            getDeleteMethod(),
            asyncUnaryCall(
              new MethodHandlers<
                rs.swir.api.client.DeleteRequest,
                rs.swir.api.client.DeleteResponse>(
                  this, METHODID_DELETE)))
          .build();
    }
  }

  /**
   */
  public static final class PersistenceApiStub extends io.grpc.stub.AbstractAsyncStub<PersistenceApiStub> {
    private PersistenceApiStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PersistenceApiStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new PersistenceApiStub(channel, callOptions);
    }

    /**
     */
    public void store(rs.swir.api.client.StoreRequest request,
        io.grpc.stub.StreamObserver<rs.swir.api.client.StoreResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(getStoreMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void retrieve(rs.swir.api.client.RetrieveRequest request,
        io.grpc.stub.StreamObserver<rs.swir.api.client.RetrieveResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(getRetrieveMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void delete(rs.swir.api.client.DeleteRequest request,
        io.grpc.stub.StreamObserver<rs.swir.api.client.DeleteResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(getDeleteMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   */
  public static final class PersistenceApiBlockingStub extends io.grpc.stub.AbstractBlockingStub<PersistenceApiBlockingStub> {
    private PersistenceApiBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PersistenceApiBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new PersistenceApiBlockingStub(channel, callOptions);
    }

    /**
     */
    public rs.swir.api.client.StoreResponse store(rs.swir.api.client.StoreRequest request) {
      return blockingUnaryCall(
          getChannel(), getStoreMethod(), getCallOptions(), request);
    }

    /**
     */
    public rs.swir.api.client.RetrieveResponse retrieve(rs.swir.api.client.RetrieveRequest request) {
      return blockingUnaryCall(
          getChannel(), getRetrieveMethod(), getCallOptions(), request);
    }

    /**
     */
    public rs.swir.api.client.DeleteResponse delete(rs.swir.api.client.DeleteRequest request) {
      return blockingUnaryCall(
          getChannel(), getDeleteMethod(), getCallOptions(), request);
    }
  }

  /**
   */
  public static final class PersistenceApiFutureStub extends io.grpc.stub.AbstractFutureStub<PersistenceApiFutureStub> {
    private PersistenceApiFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PersistenceApiFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new PersistenceApiFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<rs.swir.api.client.StoreResponse> store(
        rs.swir.api.client.StoreRequest request) {
      return futureUnaryCall(
          getChannel().newCall(getStoreMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<rs.swir.api.client.RetrieveResponse> retrieve(
        rs.swir.api.client.RetrieveRequest request) {
      return futureUnaryCall(
          getChannel().newCall(getRetrieveMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<rs.swir.api.client.DeleteResponse> delete(
        rs.swir.api.client.DeleteRequest request) {
      return futureUnaryCall(
          getChannel().newCall(getDeleteMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_STORE = 0;
  private static final int METHODID_RETRIEVE = 1;
  private static final int METHODID_DELETE = 2;

  private static final class MethodHandlers<Req, Resp> implements
      io.grpc.stub.ServerCalls.UnaryMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ServerStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ClientStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.BidiStreamingMethod<Req, Resp> {
    private final PersistenceApiImplBase serviceImpl;
    private final int methodId;

    MethodHandlers(PersistenceApiImplBase serviceImpl, int methodId) {
      this.serviceImpl = serviceImpl;
      this.methodId = methodId;
    }

    @java.lang.Override
    @java.lang.SuppressWarnings("unchecked")
    public void invoke(Req request, io.grpc.stub.StreamObserver<Resp> responseObserver) {
      switch (methodId) {
        case METHODID_STORE:
          serviceImpl.store((rs.swir.api.client.StoreRequest) request,
              (io.grpc.stub.StreamObserver<rs.swir.api.client.StoreResponse>) responseObserver);
          break;
        case METHODID_RETRIEVE:
          serviceImpl.retrieve((rs.swir.api.client.RetrieveRequest) request,
              (io.grpc.stub.StreamObserver<rs.swir.api.client.RetrieveResponse>) responseObserver);
          break;
        case METHODID_DELETE:
          serviceImpl.delete((rs.swir.api.client.DeleteRequest) request,
              (io.grpc.stub.StreamObserver<rs.swir.api.client.DeleteResponse>) responseObserver);
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
        default:
          throw new AssertionError();
      }
    }
  }

  private static abstract class PersistenceApiBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    PersistenceApiBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return rs.swir.api.client.SwirClientApiProto.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("PersistenceApi");
    }
  }

  private static final class PersistenceApiFileDescriptorSupplier
      extends PersistenceApiBaseDescriptorSupplier {
    PersistenceApiFileDescriptorSupplier() {}
  }

  private static final class PersistenceApiMethodDescriptorSupplier
      extends PersistenceApiBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final String methodName;

    PersistenceApiMethodDescriptorSupplier(String methodName) {
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
      synchronized (PersistenceApiGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new PersistenceApiFileDescriptorSupplier())
              .addMethod(getStoreMethod())
              .addMethod(getRetrieveMethod())
              .addMethod(getDeleteMethod())
              .build();
        }
      }
    }
    return result;
  }
}
