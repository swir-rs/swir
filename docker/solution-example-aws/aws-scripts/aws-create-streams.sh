./aws-delete-table.sh "swir-locks"
./aws-create-table.sh "swir-locks"
./aws-kinesis-create-stream.sh aws_processor_orders_blue 2
./aws-kinesis-create-stream.sh aws_processor_inventory_green 2
./aws-kinesis-create-stream.sh aws_processor_billing_blue 2
./aws-kinesis-create-stream.sh aws_sink_green 2
