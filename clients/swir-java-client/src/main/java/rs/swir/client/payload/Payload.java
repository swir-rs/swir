package rs.swir.client.payload;


public class Payload {
    private String name;
    private String surname;
    private int counter;


    public String getName() {
        return name;
    }

    @Override
    public String toString() {
        return "Payload{" +
                "name='" + name + '\'' +
                ", surname='" + surname + '\'' +
                ", counter=" + counter +
                '}';
    }

    public Payload setCounter(int counter) {
        this.counter = counter;
        return this;
    }

    public String getSurname() {
        return surname;
    }

    public Payload setSurname(String surname) {
        this.surname = surname;
        return this;
    }

    public Payload setName(String name) {
        this.name = name;
        return this;
    }



    public int getCounter() {
        return counter;
    }
}
