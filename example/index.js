let POOL = {}

"use strict"

const http = require("http")
const express = require("express")
const { Server } = require("ws")
const { resolve } = require("path")

const app = express()
const server = http.createServer(app)
const Ws = new Server({ server })

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
                type: "users",
                users: Object.keys(POOL)
            }))
            
            username = m.from
            POOL[username] = socket
        }
        
        m.broadcast ?
            Object.keys(POOL).filter(n => n !== username).forEach(n => POOL[n].send(packet)) :
            POOL[m.to].send(packet)
    })
})

app.get("/", (_, res) => {
    res.sendFile(
        resolve("./view/index.html")
    )
})

app.use("/*", (req, res) => {
    res.sendFile(
        resolve("./view" + req.baseUrl)
    )
})

server.listen(80)