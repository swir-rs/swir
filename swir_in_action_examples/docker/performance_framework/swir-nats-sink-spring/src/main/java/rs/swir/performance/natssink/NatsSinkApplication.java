package rs.swir.performance.natssink;

import com.google.protobuf.ByteString;
import com.sun.org.apache.xerces.internal.impl.dv.util.Base64;
import com.sun.org.apache.xerces.internal.impl.dv.util.HexBin;
import nats_msg_wrapper.NatsMsgWrapper;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;

import java.util.HashMap;
import java.util.Map;
import org.apache.commons.codec.binary.Hex;
@SpringBootApplication
public class NatsSinkApplication {
	private final static Logger logger = LoggerFactory.getLogger(rs.swir.performance.natssink.NatsSinkApplication.class);
	public static void main(String[] args) {
		SpringApplication.run(rs.swir.performance.natssink.NatsSinkApplication.class, args);
		Map<String,String> headers = new HashMap();
		headers.put("dummy","dummy");
		logger.info(String.format("%s", Hex.encodeHexString(Base64.decode("v3HCpeJ9juTqyEvnE+aTEri+rUmcbGjGgGWdFV07iMh70WtL2a3omUo19cB/Xbogptz8dLUBN8Q2tr1eCLTupg==") )));
		NatsMsgWrapper.NatsMessageWrapper msg = NatsMsgWrapper.NatsMessageWrapper.newBuilder().putAllHeaders(headers).setPayload(ByteString.copyFrom("v3HCpeJ9juTqyEvnE+aTEri+rUmcbGjGgGWdFV07iMh70WtL2a3omUo19cB/Xbogptz8dLUBN8Q2tr1eCLTupg==".getBytes())).build();
		logger.info(String.format("%s", Hex.encodeHexString( msg.toByteArray())));
		NatsMsgWrapper.NatsMessageWrapper msg2 = NatsMsgWrapper.NatsMessageWrapper.newBuilder().putAllHeaders(headers).setPayload(ByteString.copyFrom(Base64.decode("v3HCpeJ9juTqyEvnE+aTEri+rUmcbGjGgGWdFV07iMh70WtL2a3omUo19cB/Xbogptz8dLUBN8Q2tr1eCLTupg=="))).build();
		logger.info(String.format("%s", Hex.encodeHexString( msg2.toByteArray())));

//		logger.info("0a0e0a0564756d6d79120564756d6d7912efbfbd017b2270726f6475636572223a2250726f64756365546f41707044222c22636f6e73756d6572223a22537562736372696265546f41707044222c22636f756e746572223a312c2274696d657374616d70223a313539343339353938363536332c227061796c6f6164223a2242794d346f357630327a707569746c6f794e565731385271357062642f49345942414374516962514e2b664331737a3856364c324865416451522f6255467a556d2f37342b69745a4d753453687443736763634a53413d3d227d");
	}

}
