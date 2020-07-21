package rs.swir.processor;

import com.fasterxml.jackson.annotation.JsonProperty;

final class EndpointDescription {
    @JsonProperty
    String url;
    @JsonProperty("client_id")
    String clientId;

    public String getUrl() {
        return url;
    }

    public EndpointDescription setUrl(String url) {
        this.url = url;
        return this;
    }

    public String getClientId() {
        return clientId;
    }

    public EndpointDescription setClientId(String clientId) {
        this.clientId = clientId;
        return this;
    }

    @Override
    public String toString() {
        return "EndpointDescription{" +
                "url='" + url + '\'' +
                ", clientId='" + clientId + '\'' +
                '}';
    }

}
