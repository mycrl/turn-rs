const net = require("net")
const socket = net.createConnection({ port: 1935 })

setInterval(() => {
    console.log("====> send")
    socket.write("hello")
}, 1000)


socket.on("data", data => {
    console.log(data.toString())
})
