const http = require("http")
const express = require("express")
const { Server } = require("ws")
const { resolve } = require("path")

const app = express()
const server = http.createServer(app)
const Ws = new Server({ server })

app.get("/", function(_req, res) {
  res.sendFile(resolve("./index.html"))
})

global.pools = {}

Ws.on("connection", function(socket) {
  let username = null

  socket.on("close", function() {
    delete pools[username]
  })

  socket.on("error", function() {
    delete pools[username]
  })

  socket.on("message", function(packet) {
    let m = JSON.parse(packet.toString())
    if (m.type === "connected") {
      socket.send(JSON.stringify({
        users: Object.keys(pools),
        type: "users"
      }))

      username = m.from
      pools[username] = socket
    }

    if (m.broadcast) {
      Object.keys(pools)
        .filter(n => n !== username)
        .forEach(n => pools[n].send(packet))
    } else {
      pools[m.to].send(packet)
    }
  })
})

server.listen(80)