import { Proxy } from './proxy'
import { pipeline } from 'stream'
import { Decoder } from './codec'
import { Server } from './server'
import { Config } from './configure'

new Server()
    .launch(Config.port)
    .accept(socket => {
        pipeline(
            socket,
            new Decoder(),
            new Proxy(),
            socket,
            console.error
        )
    })
