kafkacat -b 127.0.0.1:9092 -C -t Request
kafkacat -P  -b localhost:9092 -H "header1=header value" -H "nullheader" -H "emptyheader=" -H "header1=duplicateIsOk" -t Response
