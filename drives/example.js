const { connect, StringCodec } = require("nats")

connect("localhost:4222").then(async nats => {
    const codec = StringCodec()
    for await (const message of nats.subscribe("auth")) {
        message.respond(codec.encode("panda"))
    }
})