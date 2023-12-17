import { TurnService, TurnObserver } from 'turn'
import dgram from 'node:dgram'

class Observer implements TurnObserver {
    constructor(private args: Args<Options>) {}

    async get_password(_addr: string, name: string) {
        return args.objects.auths[name]
    }
}

class SocketAddr {
    public address: string
    public port: number

    constructor(public source: string) {
        const [address, port] = source.split(':')
        this.port = Number(port)
        this.address = address
    }

    static from(address: string, port: number | string): SocketAddr {
        return new SocketAddr(`${address}:${port}`)
    }
}

class Args<T extends Object> {
    public objects: T

    constructor() {
        this.objects = this.parser()
    }

    parser(): T {
        let key = ''
        let args: { 
            [k: string]: string | Array<string> | { [k: string]: string } 
        } = {}

        console.log(process.argv)
        for (const item of process.argv.slice(2)) {
            if (item.startsWith('--')) {
                key = item.replace('--', '')
            } else {
                if (item.includes(',')) {
                    const values = item.split(',').map(item => {
                        if (item.length > 0) {
                            if (item.includes('=')) {
                                const [key, value] = item.split('=')
                                return { [key] : value }
                            } else {
                                return item
                            }
                        }
                    }).filter(item => {
                        return item != null
                    }) as unknown as Array<string | { [k: string]: string }>

                    args[key] = values.every(v => typeof v != 'string')
                        ? values.reduce((v, item) => Object.assign(v, item), {})
                        : values as Array<string>
                } else {
                    if (item.includes('=')) {
                        const [key, value] = item.split('=')
                        args[key] = { [key] : value }
                    } else {
                        args[key] = item
                    }
                }
            }
        }

        return args as unknown as T
    }
}

interface Options {
    port: string
    realm: string
    external: string
    auths: { [k: string]: string }
}

const args = new Args<Options>()
const socket = dgram.createSocket('udp4')
const addr = SocketAddr.from(args.objects.external, args.objects.port)
const service = new TurnService(args.objects.realm, [addr.source], new Observer(args))
const processer = service.get_processer(addr.source, args.objects.external)

socket.bind(Number(args.objects.port), () => {
    socket.on('message', async (buf, info) => {
        console.log(
            'receive udp socket message', 
            info,
        )

        try {
            const ret = await processer.process(
                buf.subarray(0, info.size), 
                SocketAddr.from(info.address, info.port).source,
            )

            if (ret != null && ret.relay) {
                const addr = new SocketAddr(ret.relay)
                socket.send(ret.data, addr.port, addr.address)
            }
        } catch {
            console.warn(
                'failed to udp message parse',
                info,
            )
        }
    })
})
