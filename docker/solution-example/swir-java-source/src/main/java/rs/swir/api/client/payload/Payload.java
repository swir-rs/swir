package rs.swir.api.client.payload;

public class Payload {
    private String producer;
    private String consumer;
    private int counter;
    private long timestamp;
    private String payload;

    public String getProducer() {
        return producer;
    }

    public Payload setProducer(String producer) {
        this.producer = producer;
        return this;
    }

    public String getConsumer() {
        return consumer;
    }

    public Payload setConsumer(String consumer) {
        this.consumer = consumer;
        return this;
    }

    public int getCounter() {
        return counter;
    }

    public Payload setCounter(int counter) {
        this.counter = counter;
        return this;
    }

    public long getTimestamp() {
        return timestamp;
    }

    public Payload setTimestamp(long timestamp) {
        this.timestamp = timestamp;
        return this;
    }

    public String getPayload() {
        return payload;
    }

    public Payload setPayload(String payload) {
        this.payload = payload;
        return this;
    }

    @Override
    public String toString() {
        return "Payload{" +
                "producer='" + producer + '\'' +
                ", consumer='" + consumer + '\'' +
                ", counter=" + counter +
                ", timestamp=" + timestamp +
                ", payload='" + payload + '\'' +
                '}';
    }
}

