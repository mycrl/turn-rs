import net from 'net'

/**
 * server
 * 
 * @module
 */

/**
 * default server options.
 * 
 * @readonly
 */
const DefaultServerOptions: net.ServerOpts = {
    allowHalfOpen: true
}

/**
 * abort panic.
 * 
 * @param err panic info.
 * @returns void
 * @private
 */
function panic(err: Error) {
    console.error(err)
    process.exit(1)
}

type Handler = (socket: net.Socket) => void

/**
 * Server
 * 
 * @class
 * @extends net.Server
 */
export class Server extends net.Server {
    
    /**
     * @constructor
     */
    constructor() {
        super(DefaultServerOptions)
    }
    
    /**
     * handle socket.
     * 
     * @param socket tcp socket.
     * @param handler tcp socket handler.
     * @returns void
     * @private
     */
    private _socket(socket: net.Socket, handler: Handler) {
        socket.setNoDelay(true)
        socket.setKeepAlive(true)
        handler(socket)
    }
    
    /**
     * tcp server listen.
     * 
     * @param port listen port.
     * @returns Server
     * @public
     */
    public launch(port: number) {
        this.listen(port)
        return this
    }
    
    /**
     * accept tco server.
     * 
     * @param handler connection event handler.
     * @returns Server
     * @public
     */
    public accept(handler: Handler) {
        this.on('connection', s => this._socket(s, handler))
        this.on('error', panic)
        return this
    }
}
