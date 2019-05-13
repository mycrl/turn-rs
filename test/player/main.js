const WebSocket = require("ws")
const http = require("http")
const express = require("lazy_mod/express")
const app = express()
// const ws = new WebSocket.Server({ server: http.createServer().listen(8080) })


app.get("/player", async function (req, res) {
    res.sendFile(__dirname + "/player.html")
})


// ws.on('connection', function connection(socket) {
//     socket.on('message', function incoming(message) {
//         console.log('received: %s', message)
//     })

//     socket.send(Buffer.from([ 0x00, 0x00, 0x00 ]))
// })


http.createServer(app).listen(8088)