const dgram = require("dgram")
const server = dgram.createSocket("udp4")

server.connect(1936, "localhost", () => {
    server.on("message", data => {
        console.log(data)
    })
})