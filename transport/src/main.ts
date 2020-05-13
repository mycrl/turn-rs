import { pipeline } from "stream"
import { Dgram } from "./dgram"
import { Decode } from "./decode"
import { Nats } from "./nats"

pipeline(
    new Dgram(1936),
    new Decode(),
    new Nats("test-cluster", "test", "nats://localhost:4222"),
    (err) => {
    console.error(err)
})
