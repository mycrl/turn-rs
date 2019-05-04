const net = require("net")


function rand (count) {
    let arrs = [];
    for (let i = 0; i < count; i ++) {
        arrs.push(Math.floor(Math.random() * 255))
    }

    return arrs
}


const client = net.createConnection({
    host: "127.0.0.1",
    port: 1935
}, () => {
    let body = [3]
    body.push(...[ 0, 0, 0, 0, 0, 0, 0, 0 ])
    body.push(...rand(1528))
    client.write(Buffer.from(body))
})


client.on("data", data => {
    console.log("接收到消息", data)
    console.log("消息长度", data.length)
})


client.on("end", () => {
    console.log("连接关闭")
})