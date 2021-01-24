const { createServer } = require("net")
const { pipeline } = require("stream")
const Mysticeti = require("../drives")

createServer(socket => {
    const mysticeti = new Mysticeti()
    
    mysticeti.bind("Auth", (auth) => {
        console.log("Auth =: ", auth)
        setTimeout(async () => {
            console.log(await mysticeti.call("Get", { addr: auth.req.addr }))
        }, 2000)
        
        return {
            password: process.argv[2],
            group: 0
        }
    })
    
    pipeline(
        socket,
        mysticeti,
        socket,
        console.error
    )
}).listen(8080)