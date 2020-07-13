package rs.swir.performance.natssink;

import org.springframework.cloud.stream.annotation.EnableBinding;
import org.springframework.cloud.stream.annotation.Input;
import org.springframework.cloud.stream.annotation.Output;
import org.springframework.context.annotation.Configuration;
import org.springframework.messaging.SubscribableChannel;

@Configuration
@EnableBinding({SinkAutoConfiguration.Topics1.class,SinkAutoConfiguration.Topics2.class})
public class SinkAutoConfiguration {
    interface Topics1{
        String INPUT = "input1";
        String OUTPUT = "output1";

        @Input(INPUT)
        SubscribableChannel input1();

        @Output(OUTPUT)
        SubscribableChannel output1();
    }
    interface Topics2{
        String INPUT = "input2";
        String OUTPUT = "output2";

        @Input(INPUT)
        SubscribableChannel input1();

        @Output(OUTPUT)
        SubscribableChannel output1();
    }
}
