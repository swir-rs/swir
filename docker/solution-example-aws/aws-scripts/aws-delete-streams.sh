./aws-delete-table.sh "swir-locks"
./aws-delete-table.sh "swir-demo"
./aws-kinesis-delete-stream.sh aws_processor_orders_blue 2
./aws-kinesis-delete-stream.sh aws_processor_inventory_green 2
./aws-kinesis-delete-stream.sh aws_processor_billing_blue 2
./aws-kinesis-delete-stream.sh aws_sink_green 2
