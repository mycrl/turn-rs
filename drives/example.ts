import { createServer } from "net"
import { pipeline } from "stream"
import Mysticeti, { Trigger, Service } from "./src"

createServer(socket => {
    const mysticeti = new Mysticeti()
    
    mysticeti.bind(Trigger.Auth, (auth) => {
        console.log("Auth =: ", auth)
        
        setTimeout(async () => {
            console.log(auth.username, await mysticeti.call(Service.Get, {
                addr: auth.addr
            }))
        }, 1000)
        
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
