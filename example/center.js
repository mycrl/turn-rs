let PORT = 49152

"use strict"

const http = require("http")
const express = require("express")

const app = express()
const server = http.createServer(app)

app.get("/", (req, res) => {
    console.dir(req.query)
    res.send({
        password: process.argv[2],
        port: PORT,
        group: 0,
    })
    
    PORT += 1
})

server.listen(8080)