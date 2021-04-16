"use strict"

import Nats from "nats"
import { EventEmitter } from "events"
import Subscription from "./subscription.js"

/** 
 * @module Mysticeti 
 */

/**
 * @extends EventEmitter
 * @class
 */
export default class Mysticeti extends EventEmitter {
    
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
     * init Mysticeti.
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
     * new Mysticeti({
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
