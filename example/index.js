let POOL = {}

"use strict"

const http = require("http")
const { v4 } = require("uuid")
const express = require("express")
const { Server } = require("ws")
const { resolve } = require("path")

const app = express()
const server = http.createServer(app)
const WsServer = new Server({ server })

WsServer.on("connection", socket => {
    const uid = v4()
    POOL[uid] = socket
    socket.on("close", () => {
        delete POOL[uid]
    })

    socket.on("error", () => {
        delete POOL[uid]
    })

    socket.on("message", packet => {
        Object.keys(POOL)
            .filter(x => x !== uid)
            .forEach(x => POOL[x].send(packet))
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