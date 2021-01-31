let POOL = {}

"use strict"

const http = require("http")
const express = require("express")
const { Server } = require("ws")
const { resolve } = require("path")

const app = express()
const server = http.createServer(app)
const Ws = new Server({ server })

app.get("/", (_, res) => {
    res.sendFile(resolve("./view/index.html"))
})

app.use("/*", (req, res) => {
    res.sendFile(resolve("./view" + req.baseUrl))
})

Ws.on("connection", socket => {
    let username = null
    
    socket.on("close", () => {
        delete POOL[username]
    })

    socket.on("error", () => {
        delete POOL[username]
    })

    socket.on("message", packet => {
        let m = JSON.parse(packet.toString())
        if (m.type === "connected") {
            socket.send(JSON.stringify({
                users: Object.keys(POOL),
                type: "users"
            }))
            
            username = m.from
            POOL[username] = socket
        }
       
        if (m.broadcast) {
            Object.keys(POOL)
                .filter(n => n !== username)
                .forEach(n => POOL[n].send(packet))
        } else {
            POOL[m.to].send(packet)
        }
    })
})

server.listen(80)