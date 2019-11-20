#Consumer
docker run -ti --rm --network="docker_default" golang
go get github.com/nats-io/go-nats-examples/tools/nats-sub
nats-sub -s "docker_nats_1:4222" ">"

#Producer
docker run -ti --rm --network="docker_default" golang
go get github.com/nats-io/go-nats-examples/tools/nats-pub
nats-pub -s "docker_nats_1:4222" Response "message from the bottle"