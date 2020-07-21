package rs.swir.processor;


import com.fasterxml.jackson.annotation.JsonProperty;

final class ClientSubscribeRequest {
    @JsonProperty()
    EndpointDescription endpoint;
    @JsonProperty("client_topic")
    String clientTopic;

    public EndpointDescription getEndpoint() {
        return endpoint;
    }

    public ClientSubscribeRequest setEndpoint(EndpointDescription endpoint) {
        this.endpoint = endpoint;
        return this;
    }

    public String getClientTopic() {
        return clientTopic;
    }

    public ClientSubscribeRequest setClientTopic(String clientTopic) {
        this.clientTopic = clientTopic;
        return this;
    }


    @Override
    public String toString() {
        return "ClientSubscribeRequest{" +
                "endpoint=" + endpoint +
                ", clientTopic='" + clientTopic + '\'' +
                '}';
    }

}
