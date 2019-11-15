docker run -ti --network="docker_default" golang
go get github.com/nats-io/go-nats-examples/tools/nats-sub
nats-sub -s "docker_nats_1:4222" ">"


go get github.com/nats-io/go-nats-examples/tools/nats-pub
nats-sub -s "docker_nats_1:4222" "subject" "message from the bottle"