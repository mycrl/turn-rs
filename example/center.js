"use strict"

const http = require("http")
const express = require("express")

const app = express()
const server = http.createServer(app)

app.get("/", (req, res) => {
  console.dir(req.query)
  res.send({
    password: process.argv[2],
    group: 0,
  })
})

server.listen(8080)
