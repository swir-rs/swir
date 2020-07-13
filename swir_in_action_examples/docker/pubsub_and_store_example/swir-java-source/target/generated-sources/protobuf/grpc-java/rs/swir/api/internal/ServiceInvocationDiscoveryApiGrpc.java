package rs.swir.api.internal;

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
    comments = "Source: internal_api.proto")
public final class ServiceInvocationDiscoveryApiGrpc {

  private ServiceInvocationDiscoveryApiGrpc() {}

  public static final String SERVICE_NAME = "swir_internal.ServiceInvocationDiscoveryApi";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<swir_common.CommonStructs.InvokeRequest,
      swir_common.CommonStructs.InvokeResponse> getInvokeMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "Invoke",
      requestType = swir_common.CommonStructs.InvokeRequest.class,
      responseType = swir_common.CommonStructs.InvokeResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<swir_common.CommonStructs.InvokeRequest,
      swir_common.CommonStructs.InvokeResponse> getInvokeMethod() {
    io.grpc.MethodDescriptor<swir_common.CommonStructs.InvokeRequest, swir_common.CommonStructs.InvokeResponse> getInvokeMethod;
    if ((getInvokeMethod = ServiceInvocationDiscoveryApiGrpc.getInvokeMethod) == null) {
      synchronized (ServiceInvocationDiscoveryApiGrpc.class) {
        if ((getInvokeMethod = ServiceInvocationDiscoveryApiGrpc.getInvokeMethod) == null) {
          ServiceInvocationDiscoveryApiGrpc.getInvokeMethod = getInvokeMethod =
              io.grpc.MethodDescriptor.<swir_common.CommonStructs.InvokeRequest, swir_common.CommonStructs.InvokeResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "Invoke"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  swir_common.CommonStructs.InvokeRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  swir_common.CommonStructs.InvokeResponse.getDefaultInstance()))
              .setSchemaDescriptor(new ServiceInvocationDiscoveryApiMethodDescriptorSupplier("Invoke"))
              .build();
        }
      }
    }
    return getInvokeMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static ServiceInvocationDiscoveryApiStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<ServiceInvocationDiscoveryApiStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<ServiceInvocationDiscoveryApiStub>() {
        @java.lang.Override
        public ServiceInvocationDiscoveryApiStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new ServiceInvocationDiscoveryApiStub(channel, callOptions);
        }
      };
    return ServiceInvocationDiscoveryApiStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static ServiceInvocationDiscoveryApiBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<ServiceInvocationDiscoveryApiBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<ServiceInvocationDiscoveryApiBlockingStub>() {
        @java.lang.Override
        public ServiceInvocationDiscoveryApiBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new ServiceInvocationDiscoveryApiBlockingStub(channel, callOptions);
        }
      };
    return ServiceInvocationDiscoveryApiBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static ServiceInvocationDiscoveryApiFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<ServiceInvocationDiscoveryApiFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<ServiceInvocationDiscoveryApiFutureStub>() {
        @java.lang.Override
        public ServiceInvocationDiscoveryApiFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new ServiceInvocationDiscoveryApiFutureStub(channel, callOptions);
        }
      };
    return ServiceInvocationDiscoveryApiFutureStub.newStub(factory, channel);
  }

  /**
   */
  public static abstract class ServiceInvocationDiscoveryApiImplBase implements io.grpc.BindableService {

    /**
     */
    public void invoke(swir_common.CommonStructs.InvokeRequest request,
        io.grpc.stub.StreamObserver<swir_common.CommonStructs.InvokeResponse> responseObserver) {
      asyncUnimplementedUnaryCall(getInvokeMethod(), responseObserver);
    }

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return io.grpc.ServerServiceDefinition.builder(getServiceDescriptor())
          .addMethod(
            getInvokeMethod(),
            asyncUnaryCall(
              new MethodHandlers<
                swir_common.CommonStructs.InvokeRequest,
                swir_common.CommonStructs.InvokeResponse>(
                  this, METHODID_INVOKE)))
          .build();
    }
  }

  /**
   */
  public static final class ServiceInvocationDiscoveryApiStub extends io.grpc.stub.AbstractAsyncStub<ServiceInvocationDiscoveryApiStub> {
    private ServiceInvocationDiscoveryApiStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected ServiceInvocationDiscoveryApiStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new ServiceInvocationDiscoveryApiStub(channel, callOptions);
    }

    /**
     */
    public void invoke(swir_common.CommonStructs.InvokeRequest request,
        io.grpc.stub.StreamObserver<swir_common.CommonStructs.InvokeResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(getInvokeMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   */
  public static final class ServiceInvocationDiscoveryApiBlockingStub extends io.grpc.stub.AbstractBlockingStub<ServiceInvocationDiscoveryApiBlockingStub> {
    private ServiceInvocationDiscoveryApiBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected ServiceInvocationDiscoveryApiBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new ServiceInvocationDiscoveryApiBlockingStub(channel, callOptions);
    }

    /**
     */
    public swir_common.CommonStructs.InvokeResponse invoke(swir_common.CommonStructs.InvokeRequest request) {
      return blockingUnaryCall(
          getChannel(), getInvokeMethod(), getCallOptions(), request);
    }
  }

  /**
   */
  public static final class ServiceInvocationDiscoveryApiFutureStub extends io.grpc.stub.AbstractFutureStub<ServiceInvocationDiscoveryApiFutureStub> {
    private ServiceInvocationDiscoveryApiFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected ServiceInvocationDiscoveryApiFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new ServiceInvocationDiscoveryApiFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<swir_common.CommonStructs.InvokeResponse> invoke(
        swir_common.CommonStructs.InvokeRequest request) {
      return futureUnaryCall(
          getChannel().newCall(getInvokeMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_INVOKE = 0;

  private static final class MethodHandlers<Req, Resp> implements
      io.grpc.stub.ServerCalls.UnaryMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ServerStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ClientStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.BidiStreamingMethod<Req, Resp> {
    private final ServiceInvocationDiscoveryApiImplBase serviceImpl;
    private final int methodId;

    MethodHandlers(ServiceInvocationDiscoveryApiImplBase serviceImpl, int methodId) {
      this.serviceImpl = serviceImpl;
      this.methodId = methodId;
    }

    @java.lang.Override
    @java.lang.SuppressWarnings("unchecked")
    public void invoke(Req request, io.grpc.stub.StreamObserver<Resp> responseObserver) {
      switch (methodId) {
        case METHODID_INVOKE:
          serviceImpl.invoke((swir_common.CommonStructs.InvokeRequest) request,
              (io.grpc.stub.StreamObserver<swir_common.CommonStructs.InvokeResponse>) responseObserver);
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

  private static abstract class ServiceInvocationDiscoveryApiBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    ServiceInvocationDiscoveryApiBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return rs.swir.api.internal.SwirInternalApiProto.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("ServiceInvocationDiscoveryApi");
    }
  }

  private static final class ServiceInvocationDiscoveryApiFileDescriptorSupplier
      extends ServiceInvocationDiscoveryApiBaseDescriptorSupplier {
    ServiceInvocationDiscoveryApiFileDescriptorSupplier() {}
  }

  private static final class ServiceInvocationDiscoveryApiMethodDescriptorSupplier
      extends ServiceInvocationDiscoveryApiBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final String methodName;

    ServiceInvocationDiscoveryApiMethodDescriptorSupplier(String methodName) {
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
      synchronized (ServiceInvocationDiscoveryApiGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new ServiceInvocationDiscoveryApiFileDescriptorSupplier())
              .addMethod(getInvokeMethod())
              .build();
        }
      }
    }
    return result;
  }
}
