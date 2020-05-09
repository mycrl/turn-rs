const dgram = require("dgram")
const server = dgram.createSocket("udp4")

server.on("message", data => {
    console.log(data)
})

server.bind(1936)