"use strict"

import Nats from "nats"
import { EventEmitter } from "events"
import Subscription from "./subscription.js"

/** 
 * @module mystical 
 */

/**
 * @extends EventEmitter
 * @class
 */
export default class mystical extends EventEmitter {
    
    /**
     * @param {strings} options.server - nats server url.
     * @constructor
     */
    constructor(options) {
        super()
        this._inner = null
        this._proxy = null
        this._options = options
        this._init()
    }
    
    /**
     * init mystical.
     * @returns {Promise<void>}
     * @private
     */
    async _init() {
        const { servers } = this._options
        this._inner = await Nats.connect({ servers })
        this.emit("ready", undefined)
    }
    
    /**
     * @public
     * @example
     * new mystical({
     *     server: "localhost:4222"
     * }).Broker.auth
     */
    get Broker() {
        if (this._proxy) return this._proxy
        return this._proxy = new Proxy({}, {
            get: (_, key) => new Subscription(
                this._inner.subscribe(key + ".>")
            )
        })
    }
}
