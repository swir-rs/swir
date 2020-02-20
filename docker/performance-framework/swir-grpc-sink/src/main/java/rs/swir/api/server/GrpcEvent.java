package rs.swir.api.server;

import java.util.Arrays;

class GrpcEvent {
    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        GrpcEvent grpcEvent = (GrpcEvent) o;
        return Arrays.equals(payload, grpcEvent.payload);
    }

    public byte[] getPayload() {
        return payload;
    }

    public GrpcEvent setPayload(byte[] payload) {
        this.payload = payload;
        return this;
    }

    @Override
    public int hashCode() {
        return Arrays.hashCode(payload);
    }

    byte[] payload;
}
